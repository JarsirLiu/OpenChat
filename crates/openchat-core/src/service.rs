use std::sync::Arc;

use openchat_infra::sqlite::{PersistedSession, SqliteChatStore};
use tokio::sync::broadcast;

use crate::{
    build_session_context,
    events::{MessageSnapshotDto, ToolCallSummaryDto},
    normalize_session_history, parse_media_assets_json, ChatRequest, ChatServiceError,
    ActiveTurnRegistry, InMemorySessionStore, SessionContext, SessionRuntime, StreamEventPayload,
    TurnAccepted, TurnPlan,
};

pub trait TurnBuilder: Send + Sync {
    fn build_turn(
        &self,
        request: ChatRequest,
        context: SessionContext,
    ) -> Result<TurnPlan, ChatServiceError>;
}

#[derive(Clone)]
pub struct ChatService {
    session_store: Arc<InMemorySessionStore>,
    active_turns: Arc<ActiveTurnRegistry>,
    chat_store: Arc<SqliteChatStore>,
    turn_builder: Arc<dyn TurnBuilder>,
    runtime: Arc<dyn TurnRunner>,
}

pub trait TurnRunner: Send + Sync {
    fn spawn_run(
        &self,
        plan: TurnPlan,
        session_runtime: SessionRuntime,
        active_turn: crate::ActiveTurnHandle,
    );
}

impl ChatService {
    pub fn new(
        session_store: Arc<InMemorySessionStore>,
        active_turns: Arc<ActiveTurnRegistry>,
        chat_store: Arc<SqliteChatStore>,
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
        let context = self
            .load_session_history(request.session_id.as_str())
            .await?;
        let plan = self.turn_builder.build_turn(request, context)?;
        let session_id = plan.session_id.clone();
        self.chat_store
            .ensure_session(session_id.as_str())
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?;
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
        let active_turn = self.active_turns.register(session_id.as_str(), turn_id.as_str()).await;
        let accepted = TurnAccepted::new(session_id.clone(), turn_id);

        self.runtime.spawn_run(plan, session_runtime, active_turn);

        Ok(accepted)
    }

    pub async fn interrupt_turn(
        &self,
        session_id: &str,
        turn_id: &str,
    ) -> Result<bool, ChatServiceError> {
        if self.active_turns.interrupt(turn_id).await {
            return Ok(true);
        }

        self.chat_store
            .interrupt_running_turn(session_id, turn_id)
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))
    }

    pub async fn subscribe(&self, session_id: &str) -> broadcast::Receiver<StreamEventPayload> {
        self.session_store
            .session_runtime(session_id)
            .await
            .sender
            .subscribe()
    }

    pub async fn list_sessions(&self) -> Result<Vec<PersistedSession>, ChatServiceError> {
        self.chat_store
            .list_sessions()
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))
    }

    pub async fn get_session(
        &self,
        session_id: &str,
    ) -> Result<Option<PersistedSession>, ChatServiceError> {
        self.reconcile_session_runtime_state(session_id).await?;
        self.chat_store
            .get_session(session_id)
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<bool, ChatServiceError> {
        let deleted = self
            .chat_store
            .delete_session(session_id)
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?;

        if deleted {
            self.session_store.remove_session(session_id).await;
        }

        Ok(deleted)
    }

    pub async fn rename_session(
        &self,
        session_id: &str,
        title: &str,
    ) -> Result<Option<PersistedSession>, ChatServiceError> {
        self.chat_store
            .update_session_title(session_id, title)
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))
    }

    pub async fn session_messages_snapshot(
        &self,
        session_id: &str,
    ) -> Result<Vec<MessageSnapshotDto>, ChatServiceError> {
        self.reconcile_session_runtime_state(session_id).await?;

        let messages = self
            .chat_store
            .list_session_messages(session_id)
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?;
        let tool_calls = self
            .chat_store
            .list_session_tool_calls(session_id)
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?;

        Ok(messages
            .into_iter()
            .map(|message| {
                let attached_tool_calls = if message.role == "assistant" {
                    let items: Vec<_> = tool_calls
                        .iter()
                        .filter(|tool_call| {
                            tool_call.parent_item_id.as_deref() == Some(message.id.as_str())
                                || (tool_call.parent_item_id.is_none()
                                    && tool_call.turn_id == message.turn_id)
                        })
                        .map(|tool_call| {
                            let media = parse_media_assets_json(tool_call.media_json.as_deref());
                            ToolCallSummaryDto {
                                media: (!media.is_empty()).then_some(media),
                                id: tool_call.id.clone(),
                                name: tool_call.tool_name.clone(),
                                display_name: tool_call.tool_display_name.clone(),
                                parent_item_id: tool_call
                                    .parent_item_id
                                    .clone()
                                    .or_else(|| Some(message.id.clone())),
                                arguments_text: tool_call.arguments_text.clone(),
                                result: tool_call
                                    .result_json
                                    .as_deref()
                                    .and_then(|value| serde_json::from_str(value).ok()),
                                status: Some(tool_call.status.clone()),
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
            .collect())
    }

    async fn load_session_history(
        &self,
        session_id: &str,
    ) -> Result<SessionContext, ChatServiceError> {
        self.reconcile_session_runtime_state(session_id).await?;

        let messages = self
            .chat_store
            .list_session_messages(session_id)
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?;
        let tool_calls = self
            .chat_store
            .list_session_tool_calls(session_id)
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?;

        let history = normalize_session_history(messages, tool_calls);

        Ok(build_session_context(session_id.to_string(), history))
    }

    async fn reconcile_session_runtime_state(
        &self,
        session_id: &str,
    ) -> Result<(), ChatServiceError> {
        let active_turn_ids = self.active_turns.active_turn_ids_for_session(session_id).await;
        self.chat_store
            .reconcile_session_runtime_state(session_id, active_turn_ids.as_slice())
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))
    }
}
