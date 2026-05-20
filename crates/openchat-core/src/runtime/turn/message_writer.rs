use openchat_infra::stores::{ChatStore, PersistedMessage};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::assistant_text_to_content_json;

#[derive(Clone)]
pub(crate) struct MessageWriter {
    chat_store: Arc<ChatStore>,
}

impl MessageWriter {
    pub fn new(chat_store: Arc<ChatStore>) -> Self {
        Self { chat_store }
    }

    pub async fn write_user_completed(
        &self,
        user_item_id: &str,
        user_id: &str,
        session_id: &str,
        turn_id: &str,
        user_content: &Value,
    ) {
        let _ = self
            .upsert(PersistedMessage {
                id: user_item_id.to_string(),
                user_id: user_id.to_string(),
                session_id: session_id.to_string(),
                turn_id: turn_id.to_string(),
                role: "user".into(),
                status: "completed".into(),
                content_json: user_content.to_string(),
                tool_call_id: None,
            })
            .await;
    }

    pub async fn write_assistant_started(
        &self,
        assistant_item_id: &str,
        user_id: &str,
        session_id: &str,
        turn_id: &str,
    ) {
        let _ = self
            .upsert(PersistedMessage {
                id: assistant_item_id.to_string(),
                user_id: user_id.to_string(),
                session_id: session_id.to_string(),
                turn_id: turn_id.to_string(),
                role: "assistant".into(),
                status: "in_progress".into(),
                content_json: assistant_text_to_content_json("").to_string(),
                tool_call_id: None,
            })
            .await;
    }

    pub async fn write_assistant_in_progress(
        &self,
        assistant_item_id: &str,
        user_id: &str,
        session_id: &str,
        turn_id: &str,
        assistant_text: &str,
    ) {
        let _ = self
            .upsert(PersistedMessage {
                id: assistant_item_id.to_string(),
                user_id: user_id.to_string(),
                session_id: session_id.to_string(),
                turn_id: turn_id.to_string(),
                role: "assistant".into(),
                status: "in_progress".into(),
                content_json: assistant_text_to_content_json(assistant_text).to_string(),
                tool_call_id: None,
            })
            .await;
    }

    pub async fn write_assistant_completed(
        &self,
        assistant_item_id: &str,
        user_id: &str,
        session_id: &str,
        turn_id: &str,
        assistant_text: &str,
    ) {
        let _ = self
            .upsert(PersistedMessage {
                id: assistant_item_id.to_string(),
                user_id: user_id.to_string(),
                session_id: session_id.to_string(),
                turn_id: turn_id.to_string(),
                role: "assistant".into(),
                status: "completed".into(),
                content_json: assistant_text_to_content_json(assistant_text).to_string(),
                tool_call_id: None,
            })
            .await;
    }

    pub async fn write_reasoning_started(
        &self,
        reasoning_item_id: &str,
        user_id: &str,
        session_id: &str,
        turn_id: &str,
    ) {
        let _ = self
            .upsert(PersistedMessage {
                id: reasoning_item_id.to_string(),
                user_id: user_id.to_string(),
                session_id: session_id.to_string(),
                turn_id: turn_id.to_string(),
                role: "reasoning".into(),
                status: "in_progress".into(),
                content_json: json!([{ "type": "text", "text": "" }]).to_string(),
                tool_call_id: None,
            })
            .await;
    }

    pub async fn write_reasoning_in_progress(
        &self,
        reasoning_item_id: &str,
        user_id: &str,
        session_id: &str,
        turn_id: &str,
        reasoning_text: &str,
    ) {
        let _ = self
            .upsert(PersistedMessage {
                id: reasoning_item_id.to_string(),
                user_id: user_id.to_string(),
                session_id: session_id.to_string(),
                turn_id: turn_id.to_string(),
                role: "reasoning".into(),
                status: "in_progress".into(),
                content_json: json!([{ "type": "text", "text": reasoning_text }]).to_string(),
                tool_call_id: None,
            })
            .await;
    }

    pub async fn write_reasoning_completed(
        &self,
        reasoning_item_id: &str,
        user_id: &str,
        session_id: &str,
        turn_id: &str,
        reasoning_text: &str,
    ) {
        let _ = self
            .upsert(PersistedMessage {
                id: reasoning_item_id.to_string(),
                user_id: user_id.to_string(),
                session_id: session_id.to_string(),
                turn_id: turn_id.to_string(),
                role: "reasoning".into(),
                status: "completed".into(),
                content_json: json!([{ "type": "text", "text": reasoning_text }]).to_string(),
                tool_call_id: None,
            })
            .await;
    }

    async fn upsert(&self, message: PersistedMessage) -> anyhow::Result<()> {
        self.chat_store.upsert_message(message).await
    }
}
