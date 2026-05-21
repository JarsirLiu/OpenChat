use anyhow::{Context as AnyhowContext, Result};
use std::sync::Arc;

use openchat_infra::stores::{ChatStore, PersistedThreadItem};
use serde_json::{json, Value};

use crate::{
    assistant_text_to_content_json,
    runtime::turn::{
        event_builder::{
            build_message_delta_event, build_message_item, build_message_started_event,
            build_reasoning_completed_event, build_reasoning_delta_event, build_reasoning_item,
            build_reasoning_started_event, build_tool_call_arguments_delta_event,
            build_tool_call_completed_event, build_tool_call_item, build_tool_call_started_event,
            send_event,
        },
        helpers::{now_millis, now_string},
        loop_step_result::CompletedToolCall,
    },
    tool_result_to_content_json, OutboundToolResult, SessionRuntime,
};

#[derive(Clone, Copy)]
pub(crate) struct ProjectionContext<'a> {
    pub user_id: &'a str,
    pub session_id: &'a str,
    pub turn_id: &'a str,
}

#[derive(Clone)]
pub(crate) struct TranscriptProjector {
    chat_store: Arc<ChatStore>,
}

impl TranscriptProjector {
    pub fn new(chat_store: Arc<ChatStore>) -> Self {
        Self { chat_store }
    }

    pub fn new_assistant_item_id(&self) -> String {
        format!("item_assistant_{}", now_millis())
    }

    pub async fn project_user_message(
        &self,
        session_runtime: &SessionRuntime,
        context: ProjectionContext<'_>,
        user_item_id: &str,
        prompt: &str,
        user_content: &Value,
    ) -> Result<()> {
        let persisted_at = now_millis().to_string();
        self.upsert_thread_item(PersistedThreadItem {
            id: user_item_id.to_string(),
            user_id: context.user_id.to_string(),
            session_id: context.session_id.to_string(),
            turn_id: context.turn_id.to_string(),
            item_type: "userMessage".into(),
            status: "completed".into(),
            seq: Some(10),
            parent_id: None,
            content_json: Some(user_content.to_string()),
            text: None,
            prompt: None,
            revised_prompt: None,
            model: None,
            size: None,
            quality: None,
            count: None,
            source_tool_call_id: None,
            source_tool_name: None,
            images_json: None,
            created_at: persisted_at.clone(),
            updated_at: persisted_at,
        })
        .await
        .context("failed to persist user transcript item")?;

        let _ = send_event(
            session_runtime,
            &build_message_started_event(
                context.session_id.to_string(),
                context.turn_id.to_string(),
                user_item_id.to_string(),
                now_string(),
                build_message_item(
                    user_item_id.to_string(),
                    context.turn_id.to_string(),
                    "completed",
                    "user",
                    Some(prompt.to_string()),
                    Some(user_content.clone()),
                ),
            ),
        );

        Ok(())
    }

    pub async fn project_reasoning_started(
        &self,
        session_runtime: &SessionRuntime,
        context: ProjectionContext<'_>,
        reasoning_item_id: &str,
    ) -> Result<()> {
        let persisted_at = now_millis().to_string();
        self.upsert_thread_item(PersistedThreadItem {
            id: reasoning_item_id.to_string(),
            user_id: context.user_id.to_string(),
            session_id: context.session_id.to_string(),
            turn_id: context.turn_id.to_string(),
            item_type: "reasoning".into(),
            status: "in_progress".into(),
            seq: Some(20),
            parent_id: None,
            content_json: Some(json!([{ "type": "text", "text": "" }]).to_string()),
            text: Some(String::new()),
            prompt: None,
            revised_prompt: None,
            model: None,
            size: None,
            quality: None,
            count: None,
            source_tool_call_id: None,
            source_tool_name: None,
            images_json: None,
            created_at: persisted_at.clone(),
            updated_at: persisted_at,
        })
        .await
        .context("failed to persist reasoning start item")?;

        let _ = send_event(
            session_runtime,
            &build_reasoning_started_event(
                context.session_id.to_string(),
                context.turn_id.to_string(),
                reasoning_item_id.to_string(),
                now_string(),
                build_reasoning_item(
                    reasoning_item_id.to_string(),
                    context.turn_id.to_string(),
                    "in_progress",
                    Some(String::new()),
                ),
            ),
        );

        Ok(())
    }

    pub async fn project_reasoning_delta(
        &self,
        session_runtime: &SessionRuntime,
        context: ProjectionContext<'_>,
        reasoning_item_id: &str,
        delta: &str,
        reasoning_text: &str,
    ) -> Result<()> {
        let persisted_at = now_millis().to_string();
        self.upsert_thread_item(PersistedThreadItem {
            id: reasoning_item_id.to_string(),
            user_id: context.user_id.to_string(),
            session_id: context.session_id.to_string(),
            turn_id: context.turn_id.to_string(),
            item_type: "reasoning".into(),
            status: "in_progress".into(),
            seq: Some(20),
            parent_id: None,
            content_json: Some(json!([{ "type": "text", "text": reasoning_text }]).to_string()),
            text: Some(reasoning_text.to_string()),
            prompt: None,
            revised_prompt: None,
            model: None,
            size: None,
            quality: None,
            count: None,
            source_tool_call_id: None,
            source_tool_name: None,
            images_json: None,
            created_at: persisted_at.clone(),
            updated_at: persisted_at,
        })
        .await
        .context("failed to persist reasoning delta item")?;

        let _ = send_event(
            session_runtime,
            &build_reasoning_delta_event(
                context.session_id.to_string(),
                context.turn_id.to_string(),
                reasoning_item_id.to_string(),
                now_string(),
                delta.to_string(),
            ),
        );

        Ok(())
    }

    pub async fn project_reasoning_completed(
        &self,
        session_runtime: &SessionRuntime,
        context: ProjectionContext<'_>,
        reasoning_item_id: &str,
        reasoning_text: &str,
    ) -> Result<()> {
        let persisted_at = now_millis().to_string();
        self.upsert_thread_item(PersistedThreadItem {
            id: reasoning_item_id.to_string(),
            user_id: context.user_id.to_string(),
            session_id: context.session_id.to_string(),
            turn_id: context.turn_id.to_string(),
            item_type: "reasoning".into(),
            status: "completed".into(),
            seq: Some(20),
            parent_id: None,
            content_json: Some(json!([{ "type": "text", "text": reasoning_text }]).to_string()),
            text: Some(reasoning_text.to_string()),
            prompt: None,
            revised_prompt: None,
            model: None,
            size: None,
            quality: None,
            count: None,
            source_tool_call_id: None,
            source_tool_name: None,
            images_json: None,
            created_at: persisted_at.clone(),
            updated_at: persisted_at,
        })
        .await
        .context("failed to persist reasoning completion item")?;

        let _ = send_event(
            session_runtime,
            &build_reasoning_completed_event(
                context.session_id.to_string(),
                context.turn_id.to_string(),
                reasoning_item_id.to_string(),
                now_string(),
                build_reasoning_item(
                    reasoning_item_id.to_string(),
                    context.turn_id.to_string(),
                    "completed",
                    Some(reasoning_text.to_string()),
                ),
            ),
        );

        Ok(())
    }

    pub async fn project_assistant_started(
        &self,
        session_runtime: &SessionRuntime,
        context: ProjectionContext<'_>,
        assistant_item_id: &str,
    ) -> Result<()> {
        let persisted_at = now_millis().to_string();
        self.upsert_thread_item(PersistedThreadItem {
            id: assistant_item_id.to_string(),
            user_id: context.user_id.to_string(),
            session_id: context.session_id.to_string(),
            turn_id: context.turn_id.to_string(),
            item_type: "agentMessage".into(),
            status: "in_progress".into(),
            seq: None,
            parent_id: None,
            content_json: Some(assistant_text_to_content_json("").to_string()),
            text: Some(String::new()),
            prompt: None,
            revised_prompt: None,
            model: None,
            size: None,
            quality: None,
            count: None,
            source_tool_call_id: None,
            source_tool_name: None,
            images_json: None,
            created_at: persisted_at.clone(),
            updated_at: persisted_at,
        })
        .await
        .context("failed to persist assistant start item")?;

        let _ = send_event(
            session_runtime,
            &build_message_started_event(
                context.session_id.to_string(),
                context.turn_id.to_string(),
                assistant_item_id.to_string(),
                now_string(),
                build_message_item(
                    assistant_item_id.to_string(),
                    context.turn_id.to_string(),
                    "in_progress",
                    "assistant",
                    Some(String::new()),
                    None,
                ),
            ),
        );

        Ok(())
    }

    pub async fn project_assistant_delta(
        &self,
        session_runtime: &SessionRuntime,
        context: ProjectionContext<'_>,
        assistant_item_id: &str,
        delta: &str,
        assistant_text: &str,
    ) -> Result<()> {
        let persisted_at = now_millis().to_string();
        self.upsert_thread_item(PersistedThreadItem {
            id: assistant_item_id.to_string(),
            user_id: context.user_id.to_string(),
            session_id: context.session_id.to_string(),
            turn_id: context.turn_id.to_string(),
            item_type: "agentMessage".into(),
            status: "in_progress".into(),
            seq: None,
            parent_id: None,
            content_json: Some(assistant_text_to_content_json(assistant_text).to_string()),
            text: Some(assistant_text.to_string()),
            prompt: None,
            revised_prompt: None,
            model: None,
            size: None,
            quality: None,
            count: None,
            source_tool_call_id: None,
            source_tool_name: None,
            images_json: None,
            created_at: persisted_at.clone(),
            updated_at: persisted_at,
        })
        .await
        .context("failed to persist assistant delta item")?;

        let _ = send_event(
            session_runtime,
            &build_message_delta_event(
                context.session_id.to_string(),
                context.turn_id.to_string(),
                assistant_item_id.to_string(),
                now_string(),
                delta.to_string(),
            ),
        );

        Ok(())
    }

    pub async fn project_tool_call_started(
        &self,
        session_runtime: &SessionRuntime,
        context: ProjectionContext<'_>,
        tool_call_id: &str,
        parent_item_id: Option<String>,
        tool_name: &str,
        is_image_generation: bool,
        arguments: Option<Value>,
    ) -> Result<()> {
        if !is_image_generation {
            let _ = send_event(
                session_runtime,
                &build_tool_call_started_event(
                    context.session_id.to_string(),
                    context.turn_id.to_string(),
                    tool_call_id.to_string(),
                    tool_call_id.to_string(),
                    parent_item_id,
                    tool_name.to_string(),
                    now_string(),
                    arguments,
                ),
            );
            return Ok(());
        }

        let prompt = arguments
            .as_ref()
            .and_then(|value| value.get("prompt"))
            .and_then(|value| value.as_str())
            .map(ToString::to_string);
        let size = arguments
            .as_ref()
            .and_then(|value| value.get("size"))
            .and_then(|value| value.as_str())
            .map(ToString::to_string);
        let quality = arguments
            .as_ref()
            .and_then(|value| value.get("quality"))
            .and_then(|value| value.as_str())
            .map(ToString::to_string);
        let count = arguments
            .as_ref()
            .and_then(|value| value.get("n"))
            .and_then(|value| value.as_i64());
        let persisted_at = now_millis().to_string();

        self.upsert_thread_item(PersistedThreadItem {
            id: format!("image:{tool_call_id}"),
            user_id: context.user_id.to_string(),
            session_id: context.session_id.to_string(),
            turn_id: context.turn_id.to_string(),
            item_type: "imageGeneration".into(),
            status: "in_progress".into(),
            seq: None,
            parent_id: parent_item_id.clone(),
            content_json: None,
            text: None,
            prompt,
            revised_prompt: None,
            model: Some(tool_name.to_string()),
            size,
            quality,
            count,
            source_tool_call_id: Some(tool_call_id.to_string()),
            source_tool_name: Some(tool_name.to_string()),
            images_json: Some("[]".into()),
            created_at: persisted_at.clone(),
            updated_at: persisted_at,
        })
        .await
        .context("failed to persist image generation start item")?;

        let _ = send_event(
            session_runtime,
            &build_tool_call_started_event(
                context.session_id.to_string(),
                context.turn_id.to_string(),
                tool_call_id.to_string(),
                tool_call_id.to_string(),
                parent_item_id,
                tool_name.to_string(),
                now_string(),
                arguments,
            ),
        );

        Ok(())
    }

    pub async fn project_tool_call_arguments_delta(
        &self,
        session_runtime: &SessionRuntime,
        context: ProjectionContext<'_>,
        tool_call_id: &str,
        parent_item_id: Option<String>,
        delta: &str,
    ) -> Result<()> {
        let _ = send_event(
            session_runtime,
            &build_tool_call_arguments_delta_event(
                context.session_id.to_string(),
                context.turn_id.to_string(),
                tool_call_id.to_string(),
                tool_call_id.to_string(),
                parent_item_id,
                now_string(),
                delta.to_string(),
            ),
        );

        Ok(())
    }

    pub async fn project_tool_call_completed(
        &self,
        session_runtime: &SessionRuntime,
        context: ProjectionContext<'_>,
        assistant_item_id: Option<&str>,
        completed: &CompletedToolCall,
        is_image_generation: bool,
    ) -> Result<()> {
        let media_has_image = completed
            .media
            .iter()
            .any(|media| media.kind == "image" && !media.url.trim().is_empty());
        if is_image_generation || media_has_image {
            let parsed_arguments =
                serde_json::from_str::<serde_json::Value>(completed.arguments_text.as_str()).ok();
            let prompt = parsed_arguments
                .as_ref()
                .and_then(|value| value.get("prompt"))
                .and_then(|value| value.as_str())
                .map(ToString::to_string);
            let size = parsed_arguments
                .as_ref()
                .and_then(|value| value.get("size"))
                .and_then(|value| value.as_str())
                .map(ToString::to_string);
            let quality = parsed_arguments
                .as_ref()
                .and_then(|value| value.get("quality"))
                .and_then(|value| value.as_str())
                .map(ToString::to_string);
            let count = parsed_arguments
                .as_ref()
                .and_then(|value| value.get("n"))
                .and_then(|value| value.as_i64());

            let images = completed
                .media
                .iter()
                .filter(|media| media.kind == "image" && !media.url.trim().is_empty())
                .map(|media| {
                    json!({
                        "url": media.url,
                        "mimeType": media.mime_type,
                        "sizeBytes": media.size_bytes,
                    })
                })
                .collect::<Vec<_>>();
            let persisted_at = now_millis().to_string();

            self.upsert_thread_item(PersistedThreadItem {
                id: format!("image:{}", completed.tool_call_id),
                user_id: context.user_id.to_string(),
                session_id: context.session_id.to_string(),
                turn_id: context.turn_id.to_string(),
                item_type: "imageGeneration".into(),
                status: if completed.failed {
                    "failed".into()
                } else {
                    "completed".into()
                },
                seq: None,
                parent_id: assistant_item_id.map(ToString::to_string),
                content_json: None,
                text: None,
                prompt,
                revised_prompt: None,
                model: Some(completed.tool_name.clone()),
                size,
                quality,
                count,
                source_tool_call_id: Some(completed.tool_call_id.clone()),
                source_tool_name: Some(completed.tool_name.clone()),
                images_json: Some(serde_json::Value::Array(images).to_string()),
                created_at: persisted_at.clone(),
                updated_at: persisted_at,
            })
            .await
            .context("failed to persist image generation completion item")?;
        }

        let _ = send_event(
            session_runtime,
            &build_tool_call_completed_event(
                context.session_id.to_string(),
                context.turn_id.to_string(),
                completed.tool_call_id.clone(),
                now_string(),
                build_tool_call_item(
                    completed.tool_call_id.clone(),
                    context.turn_id.to_string(),
                    if completed.failed {
                        "failed"
                    } else {
                        "completed"
                    },
                    completed.tool_call_id.clone(),
                    assistant_item_id.map(ToString::to_string),
                    completed.tool_name.clone(),
                    Some(completed.tool_display_name.clone()),
                    Some(completed.arguments_text.clone()),
                    tool_result_to_content_json(&OutboundToolResult {
                        tool_call_id: completed.tool_call_id.clone(),
                        tool_name: completed.tool_name.clone(),
                        tool_display_name: Some(completed.tool_display_name.clone()),
                        status: if completed.failed {
                            "failed".into()
                        } else {
                            "completed".into()
                        },
                        arguments_text: Some(completed.arguments_text.clone()),
                        result: completed.result.clone(),
                        media: completed.media.clone(),
                    }),
                ),
            ),
        );

        Ok(())
    }

    async fn upsert_thread_item(&self, item: PersistedThreadItem) -> Result<()> {
        self.chat_store.upsert_thread_item(item).await?;
        Ok(())
    }
}
