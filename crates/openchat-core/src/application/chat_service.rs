use std::sync::Arc;

use openchat_infra::stores::{PersistedSession, PersistedTurnPage};
use tokio::sync::broadcast;

use crate::{
    build_session_context, normalize_thread_item_history,
    protocol::{GeneratedImageAssetDto, ThreadItemSnapshotDto, TurnSnapshotDto},
    session_history_window_size, ActiveTurnHandle, ChatRequest, ChatServiceError, SessionContext,
    SessionRuntime, StreamEventPayload, TurnAccepted, TurnPlan,
};

use super::ports::SessionMediaManagerPort;
use super::ports::{ActiveTurnRegistryPort, ChatRepository, SessionRuntimeRegistry};

pub trait TurnBuilder: Send + Sync {
    fn build_turn(
        &self,
        request: ChatRequest,
        context: SessionContext,
    ) -> Result<TurnPlan, ChatServiceError>;
}

pub struct SessionMessagesSnapshotPage {
    pub turns: Vec<TurnSnapshotDto>,
    pub has_more: bool,
    pub next_before_turn_id: Option<String>,
}

#[derive(Clone)]
pub struct ChatService {
    session_store: Arc<dyn SessionRuntimeRegistry>,
    active_turns: Arc<dyn ActiveTurnRegistryPort>,
    chat_store: Arc<dyn ChatRepository>,
    session_media: Arc<dyn SessionMediaManagerPort>,
    turn_builder: Arc<dyn TurnBuilder>,
    runtime: Arc<dyn TurnRunner>,
}

pub trait TurnRunner: Send + Sync {
    fn spawn_run(
        &self,
        plan: TurnPlan,
        session_runtime: SessionRuntime,
        active_turn: ActiveTurnHandle,
    );
}

impl ChatService {
    pub fn new(
        session_store: Arc<dyn SessionRuntimeRegistry>,
        active_turns: Arc<dyn ActiveTurnRegistryPort>,
        chat_store: Arc<dyn ChatRepository>,
        session_media: Arc<dyn SessionMediaManagerPort>,
        turn_builder: Arc<dyn TurnBuilder>,
        runtime: Arc<dyn TurnRunner>,
    ) -> Self {
        Self {
            session_store,
            active_turns,
            chat_store,
            session_media,
            turn_builder,
            runtime,
        }
    }

    pub async fn start_turn(&self, request: ChatRequest) -> Result<TurnAccepted, ChatServiceError> {
        self.chat_store
            .ensure_session(request.user_id.as_str(), request.session_id.as_str())
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?;
        let session = self
            .chat_store
            .get_session(request.user_id.as_str(), request.session_id.as_str())
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?
            .ok_or_else(|| ChatServiceError::session_not_found("Session not found"))?;
        if session.transcript_version != "v2" {
            if let Err(error) = self
                .chat_store
                .promote_session_transcript_to_v2(
                    request.user_id.as_str(),
                    request.session_id.as_str(),
                )
                .await
            {
                let migration_error = error.to_string();
                let _ = self
                    .chat_store
                    .mark_session_transcript_migration_failed(
                        request.user_id.as_str(),
                        request.session_id.as_str(),
                        migration_error.as_str(),
                    )
                    .await;
                return Err(ChatServiceError::new(500, migration_error));
            }
        }
        let context = self
            .load_session_history(request.user_id.as_str(), request.session_id.as_str())
            .await?;
        let plan = self.turn_builder.build_turn(request, context)?;
        let session_id = plan.session_id.clone();
        let session_runtime = self
            .session_store
            .session_runtime(session_id.as_str())
            .await;
        let turn_id = format!(
            "turn_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );
        let active_turn = self
            .active_turns
            .register(session_id.as_str(), turn_id.as_str())
            .await;
        let accepted = TurnAccepted::new(session_id.clone(), turn_id);

        self.runtime.spawn_run(plan, session_runtime, active_turn);

        Ok(accepted)
    }

    pub async fn interrupt_turn(
        &self,
        user_id: &str,
        session_id: &str,
        turn_id: &str,
    ) -> Result<bool, ChatServiceError> {
        if self
            .chat_store
            .get_session(user_id, session_id)
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?
            .is_none()
        {
            return Ok(false);
        }

        if self.active_turns.interrupt(turn_id).await {
            return Ok(true);
        }

        self.chat_store
            .interrupt_running_turn(user_id, session_id, turn_id)
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))
    }

    pub async fn subscribe(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> Result<broadcast::Receiver<StreamEventPayload>, ChatServiceError> {
        if self.get_session(user_id, session_id).await?.is_none() {
            return Err(ChatServiceError::session_not_found("Session not found"));
        }

        let receiver = self
            .session_store
            .session_runtime(session_id)
            .await
            .sender
            .subscribe();

        Ok(receiver)
    }

    pub async fn list_sessions(
        &self,
        user_id: &str,
    ) -> Result<Vec<PersistedSession>, ChatServiceError> {
        self.chat_store
            .list_sessions(user_id)
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))
    }

    pub async fn get_session(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> Result<Option<PersistedSession>, ChatServiceError> {
        self.reconcile_session_runtime_state(user_id, session_id)
            .await?;
        self.chat_store
            .get_session(user_id, session_id)
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))
    }

    pub async fn delete_session(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> Result<bool, ChatServiceError> {
        if self
            .chat_store
            .get_session(user_id, session_id)
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?
            .is_none()
        {
            return Ok(false);
        }

        self.session_media
            .delete_session_media(user_id, session_id)
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?;

        let deleted = self
            .chat_store
            .delete_session(user_id, session_id)
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?;

        if deleted {
            self.session_store.remove_session(session_id).await;
        }

        Ok(deleted)
    }

    pub async fn rename_session(
        &self,
        user_id: &str,
        session_id: &str,
        title: &str,
    ) -> Result<Option<PersistedSession>, ChatServiceError> {
        self.chat_store
            .update_session_title(user_id, session_id, title)
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))
    }

    pub async fn session_messages_snapshot(
        &self,
        user_id: &str,
        session_id: &str,
        before_turn_id: Option<&str>,
    ) -> Result<SessionMessagesSnapshotPage, ChatServiceError> {
        self.get_session(user_id, session_id)
            .await?
            .ok_or_else(|| ChatServiceError::session_not_found("Session not found"))?;

        self.reconcile_session_runtime_state(user_id, session_id)
            .await?;

        let turn_page: PersistedTurnPage = self
            .chat_store
            .list_session_turns_page(
                user_id,
                session_id,
                before_turn_id,
                session_history_window_size(),
            )
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?;
        let turn_ids = turn_page
            .turns
            .iter()
            .map(|turn| turn.id.clone())
            .collect::<Vec<_>>();
        let thread_items = self
            .chat_store
            .list_session_thread_items_for_turns(user_id, session_id, turn_ids.as_slice())
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?;
        let turns = turn_page
            .turns
            .iter()
            .map(|turn| {
                let items = thread_items
                    .iter()
                    .filter(|item| item.turn_id == turn.id)
                    .map(|item| ThreadItemSnapshotDto {
                        id: item.id.clone(),
                        item_type: item.item_type.clone(),
                        session_id: item.session_id.clone(),
                        turn_id: item.turn_id.clone(),
                        status: item.status.clone(),
                        seq: item.seq.unwrap_or_default(),
                        created_at: Some(item.created_at.clone()),
                        updated_at: Some(item.updated_at.clone()),
                        parent_id: item.parent_id.clone(),
                        content: item.content_json.as_deref().and_then(|value| {
                            serde_json::from_str::<serde_json::Value>(value).ok()
                        }),
                        text: item.text.clone(),
                        prompt: item.prompt.clone(),
                        revised_prompt: item.revised_prompt.clone(),
                        model: item.model.clone(),
                        size: item.size.clone(),
                        quality: item.quality.clone(),
                        count: item.count.map(|value| value as u32),
                        source_tool_call_id: item.source_tool_call_id.clone(),
                        source_tool_name: item.source_tool_name.clone(),
                        images: item
                            .images_json
                            .as_deref()
                            .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
                            .and_then(|value| value.as_array().cloned())
                            .unwrap_or_default()
                            .iter()
                            .filter_map(|entry| {
                                let url = entry.get("url").and_then(|value| value.as_str())?;
                                let mime_type =
                                    entry.get("mimeType").and_then(|value| value.as_str())?;
                                Some(GeneratedImageAssetDto {
                                    url: url.to_string(),
                                    object_key: entry
                                        .get("objectKey")
                                        .and_then(|value| value.as_str())
                                        .map(ToString::to_string),
                                    mime_type: mime_type.to_string(),
                                    size_bytes: entry
                                        .get("sizeBytes")
                                        .and_then(|value| value.as_u64()),
                                })
                            })
                            .collect::<Vec<_>>(),
                    })
                    .collect::<Vec<_>>();

                TurnSnapshotDto {
                    id: turn.id.clone(),
                    session_id: session_id.to_string(),
                    status: turn.status.clone(),
                    started_at: Some(turn.started_at.clone()),
                    completed_at: turn.completed_at.clone(),
                    terminal_reason: turn.terminal_reason.as_ref().map(|reason| {
                        crate::protocol::TerminalReasonDto {
                            code: reason.code.clone(),
                            message: reason.message.clone(),
                        }
                    }),
                    items,
                }
            })
            .collect::<Vec<_>>();

        Ok(SessionMessagesSnapshotPage {
            turns,
            has_more: turn_page.has_more,
            next_before_turn_id: turn_page.next_before_turn_id,
        })
    }

    async fn load_session_history(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> Result<SessionContext, ChatServiceError> {
        self.get_session(user_id, session_id)
            .await?
            .ok_or_else(|| ChatServiceError::session_not_found("Session not found"))?;

        self.reconcile_session_runtime_state(user_id, session_id)
            .await?;

        let turn_page = self
            .chat_store
            .list_session_turns_page(user_id, session_id, None, session_history_window_size())
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?;
        let turn_ids = turn_page
            .turns
            .iter()
            .map(|turn| turn.id.clone())
            .collect::<Vec<_>>();
        let thread_items = self
            .chat_store
            .list_session_thread_items_for_turns(user_id, session_id, turn_ids.as_slice())
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?;
        let history = normalize_thread_item_history(thread_items);

        Ok(build_session_context(session_id.to_string(), history))
    }

    async fn reconcile_session_runtime_state(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> Result<(), ChatServiceError> {
        let active_turn_ids = self
            .active_turns
            .active_turn_ids_for_session(session_id)
            .await;
        self.chat_store
            .reconcile_session_runtime_state(user_id, session_id, active_turn_ids.as_slice())
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))
    }
}
