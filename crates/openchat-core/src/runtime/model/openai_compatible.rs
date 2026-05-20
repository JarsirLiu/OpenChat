use anyhow::Context;
use async_stream::try_stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::{sse::SseDataSource, ModelEventStream, ModelStreamEvent};
use crate::{
    ChatServiceError, ModelMediaUrlResolver, OutboundContentPart, OutboundMessage,
    format_outbound_tool_result_text,
    ResolvedTextModelAccess, ToolSpec, TurnPlan,
};

const DEFAULT_SYSTEM_PROMPT: &str = "你是 OpenChat 智能助手。请直接回答用户问题，保持自然、简洁、专业。除非用户主动询问你的身份、能力边界或系统实现，否则不要主动介绍自己，不要提及 Agent Runtime、系统提示词、工具链或内部实现。";

#[derive(Clone)]
pub struct OpenAiCompatibleRuntime {
    client: Client,
    media_url_resolver: Arc<dyn ModelMediaUrlResolver>,
}

impl OpenAiCompatibleRuntime {
    pub fn new(media_url_resolver: Arc<dyn ModelMediaUrlResolver>) -> Self {
        Self {
            client: Client::new(),
            media_url_resolver,
        }
    }

    pub async fn stream_text(
        &self,
        plan: &TurnPlan,
        access: &ResolvedTextModelAccess,
    ) -> Result<ModelEventStream, ChatServiceError> {
        let url = format!("{}/chat/completions", access.base_url.trim_end_matches('/'));
        let messages = build_openai_messages(
            plan,
            &access.input_modalities,
            self.media_url_resolver.as_ref(),
        )
        .await;

        let response = self
            .client
            .post(url)
            .bearer_auth(access.api_key.as_str())
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .json(&OpenAiChatRequest {
                model: access.model_name.clone(),
                messages,
                tools: build_openai_tools(&plan.tool_list)?,
                stream: true,
                enable_thinking: Some(true),
                stream_options: Some(OpenAiStreamOptions {
                    include_usage: true,
                }),
            })
            .send()
            .await
            .with_context(|| format!("failed to call provider `{}`", access.provider_key))
            .map_err(map_runtime_error)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(map_provider_response_error(
                access.provider_key.as_str(),
                status.as_u16(),
                body,
            ));
        }

        let byte_stream = response.bytes_stream();
        let stream = try_stream! {
            let mut source = SseDataSource::new(byte_stream);
            let mut pending_tool_calls = std::collections::BTreeMap::<usize, PendingToolCall>::new();

            while let Some(payload) = source.next_data().await? {
                if payload == "[DONE]" {
                    for (_, pending) in std::mem::take(&mut pending_tool_calls) {
                        if pending.id.is_empty() || pending.name.is_empty() {
                            continue;
                        }
                        yield ModelStreamEvent::ToolCallComplete {
                            tool_call_id: pending.id,
                            tool_name: pending.name,
                            arguments_text: pending.arguments,
                        };
                    }
                    break;
                }

                let chunk: OpenAiChatStreamChunk = match serde_json::from_str(payload.as_str()) {
                    Ok(chunk) => chunk,
                    Err(_) => {
                        continue;
                    }
                };

                if let Some(error) = chunk.error {
                    let message = error
                        .message
                        .unwrap_or_else(|| "Model streaming request failed".to_string());
                    Err(ChatServiceError::upstream(message))?;
                }

                for choice in chunk.choices {
                    if let Some(reasoning) = choice
                        .delta
                        .reasoning_content
                        .or(choice.delta.reasoning)
                        .filter(|value| !value.is_empty())
                    {
                        yield ModelStreamEvent::ReasoningDelta(reasoning);
                    }

                    if let Some(content) = choice.delta.content.filter(|value| !value.is_empty()) {
                        yield ModelStreamEvent::TextDelta(content);
                    }

                    if let Some(tool_call_deltas) = choice.delta.tool_calls {
                        for delta in tool_call_deltas {
                            let entry = pending_tool_calls.entry(delta.index).or_default();

                            if let Some(id) = delta.id.filter(|value| !value.is_empty()) {
                                entry.id = id;
                            }

                            if let Some(function) = delta.function {
                                if let Some(name) = function.name.filter(|value| !value.is_empty()) {
                                    entry.name = name;
                                }

                                if !entry.started
                                    && !entry.id.is_empty()
                                    && !entry.name.is_empty()
                                {
                                    entry.started = true;
                                    yield ModelStreamEvent::ToolCallStart {
                                        tool_call_id: entry.id.clone(),
                                        tool_name: entry.name.clone(),
                                        arguments: None,
                                    };
                                }

                                if let Some(arguments) =
                                    function.arguments.filter(|value| !value.is_empty())
                                {
                                    entry.arguments.push_str(arguments.as_str());
                                    yield ModelStreamEvent::ToolCallArgumentsDelta {
                                        tool_call_id: entry.id.clone(),
                                        delta: arguments,
                                    };
                                }
                            }
                        }
                    }

                    if matches!(choice.finish_reason.as_deref(), Some("tool_calls")) {
                        for (_, pending) in std::mem::take(&mut pending_tool_calls) {
                            if pending.id.is_empty() || pending.name.is_empty() {
                                continue;
                            }
                            yield ModelStreamEvent::ToolCallComplete {
                                tool_call_id: pending.id,
                                tool_name: pending.name,
                                arguments_text: pending.arguments,
                            };
                        }
                    }
                }
            }

            for (_, pending) in std::mem::take(&mut pending_tool_calls) {
                if pending.id.is_empty() || pending.name.is_empty() {
                    continue;
                }
                yield ModelStreamEvent::ToolCallComplete {
                    tool_call_id: pending.id,
                    tool_name: pending.name,
                    arguments_text: pending.arguments,
                };
            }
        };

        Ok(Box::pin(stream))
    }
}

async fn build_openai_messages(
    plan: &TurnPlan,
    input_modalities: &[String],
    media_url_resolver: &dyn ModelMediaUrlResolver,
) -> Vec<OpenAiMessage> {
    let supports_image_inputs = model_supports_image_inputs(input_modalities);
    let mut messages = Vec::new();
    messages.push(OpenAiMessage {
        role: "system".into(),
        content: Some(OpenAiMessageContent::Text(build_system_prompt())),
        tool_calls: None,
        tool_call_id: None,
    });

    let mut pending_reasoning_by_turn: std::collections::HashMap<&str, String> =
        std::collections::HashMap::new();

    for message in &plan.history {
        match message.role.as_str() {
            "reasoning" => {
                let reasoning = flatten_text_content(message);
                if reasoning.trim().is_empty() {
                    continue;
                }

                pending_reasoning_by_turn
                    .entry(message.turn_id.as_str())
                    .and_modify(|current| current.push_str(reasoning.as_str()))
                    .or_insert(reasoning);
            }
            "assistant" => {
                let reasoning = pending_reasoning_by_turn.remove(message.turn_id.as_str());
                let content = build_openai_message_content(
                    message,
                    reasoning.as_deref(),
                    supports_image_inputs,
                    media_url_resolver,
                )
                .await;
                let tool_calls = (!message.tool_calls.is_empty())
                    .then(|| build_openai_tool_call_messages(&message.tool_calls));

                if content.is_some() || tool_calls.is_some() {
                    messages.push(OpenAiMessage {
                        role: "assistant".into(),
                        content,
                        tool_calls,
                        tool_call_id: None,
                    });
                }
            }
            _ => {
                if let Some(content) = build_openai_message_content(
                    message,
                    None,
                    supports_image_inputs,
                    media_url_resolver,
                )
                .await
                {
                    messages.push(OpenAiMessage {
                        role: if message.role == "tool" {
                            "tool".into()
                        } else {
                            "user".into()
                        },
                        content: Some(content),
                        tool_calls: None,
                        tool_call_id: message.tool_call_id.clone(),
                    });
                }
            }
        }
    }

    if has_current_user_input(plan.prompt.as_str(), plan.attachments.as_slice()) {
        messages.push(OpenAiMessage {
            role: "user".into(),
            content: Some(
                build_current_user_message_content(
                    plan.prompt.as_str(),
                    plan.attachments.as_slice(),
                    supports_image_inputs,
                    media_url_resolver,
                )
                .await,
            ),
            tool_calls: None,
            tool_call_id: None,
        });
    }

    messages
}

fn build_system_prompt() -> String {
    DEFAULT_SYSTEM_PROMPT.to_string()
}

fn has_current_user_input(prompt: &str, attachments: &[crate::TurnAttachment]) -> bool {
    !prompt.trim().is_empty() || !attachments.is_empty()
}

fn build_openai_tools(
    tool_list: &[crate::TurnToolRef],
) -> Result<Option<Vec<ToolSpec>>, ChatServiceError> {
    if tool_list.is_empty() {
        return Ok(None);
    }

    let registry = crate::ToolRegistry;
    registry.specs_for_turn_tools(tool_list).map(Some)
}

async fn build_openai_message_content(
    message: &OutboundMessage,
    reasoning: Option<&str>,
    supports_image_inputs: bool,
    media_url_resolver: &dyn ModelMediaUrlResolver,
) -> Option<OpenAiMessageContent> {
    if supports_image_inputs {
        let mut parts = Vec::new();

        if let Some(reasoning) = reasoning.map(str::trim).filter(|value| !value.is_empty()) {
            parts.push(OpenAiContentPart::Text {
                text: format!("<think>{reasoning}</think>"),
            });
        }

        let mut has_image_part = false;
        for part in &message.content {
            match part {
                OutboundContentPart::Text { text } if !text.trim().is_empty() => {
                    parts.push(OpenAiContentPart::Text { text: text.clone() });
                }
                OutboundContentPart::ImageUrl { url, media_id } if !url.trim().is_empty() => {
                    has_image_part = true;
                    let resolved_url = match media_id.as_deref() {
                        Some(media_id) => media_url_resolver.resolve_model_url(media_id, url).await,
                        None => url.clone(),
                    };
                    parts.push(OpenAiContentPart::ImageUrl {
                        image_url: OpenAiImageUrl {
                            url: resolved_url,
                            detail: Some("auto".to_string()),
                        },
                    });
                }
                OutboundContentPart::ToolResult(tool_result) => {
                    let summary_text = format_outbound_tool_result_text(tool_result);
                    if !summary_text.trim().is_empty() {
                        parts.push(OpenAiContentPart::Text { text: summary_text });
                    }

                    for media in tool_result.media.iter().filter(|asset| asset.kind == "image") {
                        if media.url.trim().is_empty() {
                            continue;
                        }
                        has_image_part = true;
                        let resolved_url = match media.object_key.as_deref() {
                            Some(media_id) => {
                                media_url_resolver.resolve_model_url(media_id, media.url.as_str()).await
                            }
                            None => media.url.clone(),
                        };
                        parts.push(OpenAiContentPart::ImageUrl {
                            image_url: OpenAiImageUrl {
                                url: resolved_url,
                                detail: Some("auto".to_string()),
                            },
                        });
                    }
                }
                _ => {}
            }
        }

        if parts.is_empty() {
            return None;
        }

        if !has_image_part {
            let text = parts
                .into_iter()
                .filter_map(|part| match part {
                    OpenAiContentPart::Text { text } => Some(text),
                    OpenAiContentPart::ImageUrl { .. } => None,
                })
                .collect::<Vec<_>>()
                .join("\n");

            return if text.trim().is_empty() {
                None
            } else {
                Some(OpenAiMessageContent::Text(text))
            };
        }

        return Some(OpenAiMessageContent::Parts(parts));
    }

    let text = flatten_text_content(message);
    let merged = merge_assistant_turn_content(reasoning, text.as_str());
    if merged.trim().is_empty() {
        None
    } else {
        Some(OpenAiMessageContent::Text(merged))
    }
}

async fn build_current_user_message_content(
    prompt: &str,
    attachments: &[crate::TurnAttachment],
    supports_image_inputs: bool,
    media_url_resolver: &dyn ModelMediaUrlResolver,
) -> OpenAiMessageContent {
    if supports_image_inputs {
        let mut parts = Vec::new();

        if !prompt.trim().is_empty() {
            parts.push(OpenAiContentPart::Text {
                text: prompt.to_string(),
            });
        }

        for attachment in attachments {
            if attachment.mime_type.starts_with("image/") && !attachment.url.trim().is_empty() {
                parts.push(OpenAiContentPart::ImageUrl {
                    image_url: OpenAiImageUrl {
                        url: media_url_resolver
                            .resolve_model_url(attachment.id.as_str(), attachment.url.as_str())
                            .await,
                        detail: Some("auto".to_string()),
                    },
                });
            }
        }

        if parts
            .iter()
            .any(|part| matches!(part, OpenAiContentPart::ImageUrl { .. }))
        {
            return OpenAiMessageContent::Parts(parts);
        }
    }

    OpenAiMessageContent::Text(flatten_current_user_input(prompt, attachments))
}

fn flatten_text_content(message: &OutboundMessage) -> String {
    let mut parts = Vec::new();
    for part in &message.content {
        match part {
            OutboundContentPart::Text { text } => parts.push(text.clone()),
            OutboundContentPart::ImageUrl { .. } => {
                parts.push(text_only_image_placeholder().to_string())
            }
            OutboundContentPart::ToolResult(tool_result) => {
                parts.push(format_outbound_tool_result_text(tool_result));
            }
        }
    }
    parts.join("\n")
}

fn flatten_current_user_input(prompt: &str, attachments: &[crate::TurnAttachment]) -> String {
    let mut parts = Vec::new();

    if !prompt.trim().is_empty() {
        parts.push(prompt.to_string());
    }

    for attachment in attachments {
        if attachment.mime_type.starts_with("image/") {
            parts.push(text_only_image_placeholder().to_string());
        }
    }

    parts.join("\n")
}

fn text_only_image_placeholder() -> &'static str {
    "[User uploaded an image, but the selected model does not support direct image input.]"
}

fn merge_assistant_turn_content(reasoning: Option<&str>, content: &str) -> String {
    let reasoning = reasoning.unwrap_or("").trim();
    let content = content.trim();

    match (reasoning.is_empty(), content.is_empty()) {
        (true, true) => String::new(),
        (false, true) => format!("<think>{reasoning}</think>"),
        (true, false) => content.to_string(),
        (false, false) => format!("<think>{reasoning}</think>\n\n{content}"),
    }
}

fn map_runtime_error(error: anyhow::Error) -> ChatServiceError {
    ChatServiceError::upstream(error.to_string())
}

fn map_provider_response_error(
    provider_key: &str,
    status_code: u16,
    body: String,
) -> ChatServiceError {
    if matches!(status_code, 401 | 403) {
        return ChatServiceError::provider_authentication_failed(format!(
            "Provider `{provider_key}` authentication failed. Please update the API key and try again."
        ));
    }

    ChatServiceError::upstream(format!(
        "Provider `{provider_key}` request failed: {status_code} {body}"
    ))
}

#[derive(Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ToolSpec>>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_thinking: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream_options: Option<OpenAiStreamOptions>,
}

#[derive(Serialize)]
struct OpenAiStreamOptions {
    include_usage: bool,
}

#[derive(Serialize)]
struct OpenAiMessage {
    role: String,
    content: Option<OpenAiMessageContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAiToolCallMessage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum OpenAiMessageContent {
    Text(String),
    Parts(Vec<OpenAiContentPart>),
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum OpenAiContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: OpenAiImageUrl },
}

#[derive(Serialize)]
struct OpenAiImageUrl {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

#[derive(Serialize)]
struct OpenAiToolCallMessage {
    id: String,
    #[serde(rename = "type")]
    kind: &'static str,
    function: OpenAiToolCallFunction,
}

#[derive(Serialize)]
struct OpenAiToolCallFunction {
    name: String,
    arguments: String,
}

fn build_openai_tool_call_messages(
    tool_calls: &[crate::OutboundToolCall],
) -> Vec<OpenAiToolCallMessage> {
    tool_calls
        .iter()
        .map(|tool_call| OpenAiToolCallMessage {
            id: tool_call.id.clone(),
            kind: "function",
            function: OpenAiToolCallFunction {
                name: tool_call.name.clone(),
                arguments: tool_call.arguments_text.clone(),
            },
        })
        .collect()
}

fn model_supports_image_inputs(input_modalities: &[String]) -> bool {
    input_modalities.iter().any(|modality| {
        modality.eq_ignore_ascii_case("image") || modality.eq_ignore_ascii_case("vision")
    })
}

#[cfg(test)]
mod tests {
    use super::{
        build_openai_message_content, build_openai_messages, model_supports_image_inputs,
        OpenAiMessageContent,
    };
    use crate::{
        ModelMediaUrlResolver, OutboundContentPart, OutboundMessage, OutboundToolCall, TurnPlan,
    };
    use async_trait::async_trait;

    struct NoopMediaResolver;

    #[async_trait]
    impl ModelMediaUrlResolver for NoopMediaResolver {
        async fn resolve_model_url(&self, media_id: &str, fallback_url: &str) -> String {
            if media_id.trim().is_empty() {
                fallback_url.to_string()
            } else {
                format!("resolved://{media_id}")
            }
        }
    }

    #[test]
    fn image_inputs_are_detected_from_modalities() {
        assert!(model_supports_image_inputs(&[
            "text".into(),
            "image".into()
        ]));
        assert!(model_supports_image_inputs(&["vision".into()]));
        assert!(!model_supports_image_inputs(&["text".into()]));
    }

    #[tokio::test]
    async fn text_only_models_drop_image_parts_from_context() {
        let message = OutboundMessage {
            role: "assistant".into(),
            item_id: "item_1".into(),
            turn_id: "turn_1".into(),
            content: vec![
                OutboundContentPart::Text {
                    text: "hello".into(),
                },
                OutboundContentPart::ImageUrl {
                    url: "https://example.com/image.png".into(),
                    media_id: None,
                },
            ],
            tool_calls: Vec::new(),
            tool_call_id: None,
        };

        let content = build_openai_message_content(&message, None, false, &NoopMediaResolver)
            .await
            .expect("text content should remain available");

        match content {
            OpenAiMessageContent::Text(text) => {
                assert!(text.contains("hello"));
                assert!(text.contains("selected model does not support direct image input"));
            }
            OpenAiMessageContent::Parts(_) => {
                panic!("text-only models should not receive image parts")
            }
        }
    }

    #[tokio::test]
    async fn multimodal_models_keep_image_parts() {
        let message = OutboundMessage {
            role: "user".into(),
            item_id: "item_1".into(),
            turn_id: "turn_1".into(),
            content: vec![OutboundContentPart::ImageUrl {
                url: "https://example.com/image.png".into(),
                media_id: None,
            }],
            tool_calls: Vec::new(),
            tool_call_id: None,
        };

        let content = build_openai_message_content(&message, None, true, &NoopMediaResolver)
            .await
            .expect("image content should remain available");

        match content {
            OpenAiMessageContent::Parts(parts) => {
                assert!(parts
                    .iter()
                    .any(|part| matches!(part, super::OpenAiContentPart::ImageUrl { .. })));
            }
            OpenAiMessageContent::Text(_) => {
                panic!("multimodal models should receive image parts");
            }
        }
    }

    #[tokio::test]
    async fn assistant_tool_calls_are_kept_without_text_content() {
        let history = vec![OutboundMessage {
            role: "assistant".into(),
            item_id: "item_1".into(),
            turn_id: "turn_1".into(),
            content: Vec::new(),
            tool_calls: vec![OutboundToolCall {
                id: "call_1".into(),
                name: "image_generation".into(),
                arguments_text: "{\"prompt\":\"cat\"}".into(),
            }],
            tool_call_id: None,
        }];

        let messages = build_openai_messages(
            &TurnPlan {
                user_id: "user_1".into(),
                session_id: "session_1".into(),
                prompt: "".into(),
                attachments: Vec::new(),
                history,
                text_model: crate::TurnModelRef {
                    runtime_provider: "openai_compatible".into(),
                    model_config_id: "model_1".into(),
                    model_name: "gpt-test".into(),
                    display_name: "GPT Test".into(),
                    provider: "openai".into(),
                    source: "openchat".into(),
                    input_modalities: vec!["text".into()],
                },
                tool_list: Vec::new(),
            },
            &["text".into()],
            &NoopMediaResolver,
        )
        .await;

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1].role, "assistant");
        assert!(messages[1].content.is_none());
        assert!(messages[1].tool_calls.is_some());
    }

    #[tokio::test]
    async fn continuation_round_does_not_append_empty_user_message() {
        let messages = build_openai_messages(
            &TurnPlan {
                user_id: "user_1".into(),
                session_id: "session_1".into(),
                prompt: "".into(),
                attachments: Vec::new(),
                history: Vec::new(),
                text_model: crate::TurnModelRef {
                    runtime_provider: "openai_compatible".into(),
                    model_config_id: "model_1".into(),
                    model_name: "gpt-test".into(),
                    display_name: "GPT Test".into(),
                    provider: "openai".into(),
                    source: "openchat".into(),
                    input_modalities: vec!["text".into()],
                },
                tool_list: Vec::new(),
            },
            &["text".into()],
            &NoopMediaResolver,
        )
        .await;

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "system");
    }
}

#[derive(Deserialize)]
struct OpenAiChatStreamChunk {
    #[serde(default)]
    choices: Vec<OpenAiChatStreamChoice>,
    #[serde(default)]
    error: Option<OpenAiApiError>,
}

#[derive(Deserialize)]
struct OpenAiChatStreamChoice {
    #[serde(default)]
    delta: OpenAiChatStreamDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Default, Deserialize)]
struct OpenAiChatStreamDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
    #[serde(default)]
    reasoning: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAiChatStreamToolCallDelta>>,
}

#[derive(Default)]
struct PendingToolCall {
    id: String,
    name: String,
    arguments: String,
    started: bool,
}

#[derive(Deserialize)]
struct OpenAiChatStreamToolCallDelta {
    index: usize,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<OpenAiChatStreamFunctionDelta>,
}

#[derive(Deserialize)]
struct OpenAiChatStreamFunctionDelta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiApiError {
    #[serde(default)]
    message: Option<String>,
}
