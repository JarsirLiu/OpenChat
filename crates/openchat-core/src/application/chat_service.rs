use std::sync::Arc;

use openchat_infra::stores::{PersistedSession, PersistedTurnPage};
use tokio::sync::broadcast;

use crate::{
    build_session_context, collect_attached_tool_calls, normalize_session_history,
    parse_media_assets_json,
    protocol::{MessageSnapshotDto, ToolCallSummaryDto},
    session_history_window_size, tool_result_to_content_json, ActiveTurnHandle, ChatRequest,
    ChatServiceError, OutboundToolResult, SessionContext, SessionRuntime, StreamEventPayload,
    TurnAccepted, TurnPlan,
};

use super::ports::{ActiveTurnRegistryPort, ChatRepository, SessionRuntimeRegistry};

pub trait TurnBuilder: Send + Sync {
    fn build_turn(
        &self,
        request: ChatRequest,
        context: SessionContext,
    ) -> Result<TurnPlan, ChatServiceError>;
}

pub struct SessionMessagesSnapshotPage {
    pub messages: Vec<MessageSnapshotDto>,
    pub has_more: bool,
    pub next_before_turn_id: Option<String>,
}

#[derive(Clone)]
pub struct ChatService {
    session_store: Arc<dyn SessionRuntimeRegistry>,
    active_turns: Arc<dyn ActiveTurnRegistryPort>,
    chat_store: Arc<dyn ChatRepository>,
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
        turn_builder: Arc<dyn TurnBuilder>,
        runtime: Arc<dyn TurnRunner>,
    ) -> Self {
        Self {
            session_store,
            active_turns,
            chat_store,
            turn_builder,
            runtime,
        }
    }

    pub async fn start_turn(&self, request: ChatRequest) -> Result<TurnAccepted, ChatServiceError> {
        self.chat_store
            .ensure_session(request.user_id.as_str(), request.session_id.as_str())
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?;
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
        if self.get_session(user_id, session_id).await?.is_none() {
            return Err(ChatServiceError::session_not_found("Session not found"));
        }

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
        let messages = self
            .chat_store
            .list_session_messages_for_turns(user_id, session_id, turn_page.turn_ids.as_slice())
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?;
        let tool_calls = self
            .chat_store
            .list_session_tool_calls_for_turns(user_id, session_id, turn_page.turn_ids.as_slice())
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?;

        Ok(SessionMessagesSnapshotPage {
            messages: messages
                .into_iter()
                .map(|message| {
                    let attached_tool_calls = if message.role == "assistant" {
                        let items: Vec<_> = collect_attached_tool_calls(
                            message.id.as_str(),
                            message.turn_id.as_str(),
                            &tool_calls,
                        )
                        .iter()
                        .map(|tool_call| {
                            let media = parse_media_assets_json(tool_call.media_json.as_deref());
                            ToolCallSummaryDto {
                                id: tool_call.id.clone(),
                                name: tool_call.tool_name.clone(),
                                display_name: tool_call.tool_display_name.clone(),
                                parent_item_id: tool_call
                                    .parent_item_id
                                    .clone()
                                    .or_else(|| Some(message.id.clone())),
                                arguments_text: tool_call.arguments_text.clone(),
                                status: Some(tool_call.status.clone()),
                                content: tool_result_to_content_json(&OutboundToolResult {
                                    tool_call_id: tool_call.id.clone(),
                                    tool_name: tool_call.tool_name.clone(),
                                    tool_display_name: tool_call.tool_display_name.clone(),
                                    status: tool_call.status.clone(),
                                    arguments_text: tool_call.arguments_text.clone(),
                                    result: tool_call
                                        .result_json
                                        .as_deref()
                                        .and_then(|value| serde_json::from_str(value).ok())
                                        .unwrap_or_else(|| serde_json::json!({})),
                                    media,
                                }),
                            }
                        })
                        .collect();
                        if items.is_empty() {
                            None
                        } else {
                            Some(items)
                        }
                    } else {
                        None
                    };

                    MessageSnapshotDto {
                        id: message.id,
                        role: message.role,
                        turn_id: message.turn_id,
                        status: message.status,
                        created_at: message.created_at,
                        updated_at: message.updated_at,
                        content: message.content,
                        tool_calls: attached_tool_calls,
                    }
                })
                .collect(),
            has_more: turn_page.has_more,
            next_before_turn_id: turn_page.next_before_turn_id,
        })
    }

    async fn load_session_history(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> Result<SessionContext, ChatServiceError> {
        if self.get_session(user_id, session_id).await?.is_none() {
            return Err(ChatServiceError::session_not_found("Session not found"));
        }

        self.reconcile_session_runtime_state(user_id, session_id)
            .await?;

        let messages = self
            .chat_store
            .list_session_messages(user_id, session_id)
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?;
        let tool_calls = self
            .chat_store
            .list_session_tool_calls(user_id, session_id)
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?;

        let history = normalize_session_history(messages, tool_calls);

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
