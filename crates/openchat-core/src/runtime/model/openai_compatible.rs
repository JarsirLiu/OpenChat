use async_stream::try_stream;
use reqwest::Client;
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

use super::{sse::SseDataSource, ModelEventStream, ModelStreamEvent};
use crate::{
    format_outbound_tool_result_text, ChatServiceError, ModelMediaUrlResolver, OutboundContentPart,
    OutboundMessage, ResolvedTextModelAccess, ToolSpec, TurnPlan,
};

const DEFAULT_SYSTEM_PROMPT: &str = "你是 OpenChat 智能助手。请直接回答用户问题，保持自然、简洁、专业。除非用户主动询问你的身份、能力边界或系统实现，否则不要主动介绍自己，不要提及 Agent Runtime、系统提示词、工具链或内部实现。";
const UPSTREAM_RETRY_DELAY: Duration = Duration::from_millis(350);
const MAX_TEXT_SEND_ATTEMPTS: usize = 2;

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

        let request_body = OpenAiChatRequest {
            model: access.model_name.clone(),
            messages,
            tools: build_openai_tools(&plan.tool_list)?,
            stream: true,
            enable_thinking: Some(true),
            stream_options: Some(OpenAiStreamOptions {
                include_usage: true,
            }),
        };

        let response =
            send_text_request_with_retry(&self.client, url.as_str(), access, &request_body).await?;

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

async fn send_text_request_with_retry(
    client: &Client,
    url: &str,
    access: &ResolvedTextModelAccess,
    body: &OpenAiChatRequest,
) -> Result<reqwest::Response, ChatServiceError> {
    for attempt in 1..=MAX_TEXT_SEND_ATTEMPTS {
        let response = client
            .post(url)
            .bearer_auth(access.api_key.as_str())
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .json(body)
            .send()
            .await;

        match response {
            Ok(response) => return Ok(response),
            Err(error)
                if attempt < MAX_TEXT_SEND_ATTEMPTS && should_retry_transport_error(&error) =>
            {
                sleep(UPSTREAM_RETRY_DELAY).await;
            }
            Err(error) => {
                return Err(map_runtime_error(anyhow::Error::new(error).context(
                    format!("failed to call provider `{}`", access.provider_key),
                )));
            }
        }
    }

    Err(ChatServiceError::upstream(
        "text model request exhausted retry attempts",
    ))
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
                    true,
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
                    message.role != "tool",
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
    allow_image_parts: bool,
    media_url_resolver: &dyn ModelMediaUrlResolver,
) -> Option<OpenAiMessageContent> {
    if supports_image_inputs && allow_image_parts {
        let mut parts = Vec::new();
        let mut image_ref_index = 0usize;

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
                    image_ref_index += 1;
                    for line in format_image_reference_lines(
                        image_ref_index,
                        media_id.as_deref(),
                        Some(url.as_str()),
                    ) {
                        parts.push(OpenAiContentPart::Text { text: line });
                    }
                    let resolved_url = match media_id.as_deref() {
                        Some(media_id) => media_url_resolver.resolve_model_url(media_id, url).await,
                        None => url.clone(),
                    };
                    if resolved_url.trim().is_empty() {
                        continue;
                    }
                    has_image_part = true;
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

                    for media in tool_result
                        .media
                        .iter()
                        .filter(|asset| asset.kind == "image")
                    {
                        if media.url.trim().is_empty() {
                            continue;
                        }
                        has_image_part = true;
                        let resolved_url = match media.object_key.as_deref() {
                            Some(media_id) => {
                                media_url_resolver
                                    .resolve_model_url(media_id, media.url.as_str())
                                    .await
                            }
                            None => media.url.clone(),
                        };
                        if resolved_url.trim().is_empty() {
                            continue;
                        }
                        has_image_part = true;
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
        let mut image_ref_index = 0usize;

        if !prompt.trim().is_empty() {
            parts.push(OpenAiContentPart::Text {
                text: prompt.to_string(),
            });
        }

        for attachment in attachments {
            if attachment.mime_type.starts_with("image/") && !attachment.url.trim().is_empty() {
                image_ref_index += 1;
                for line in format_image_reference_lines(
                    image_ref_index,
                    Some(attachment.id.as_str()),
                    Some(attachment.url.as_str()),
                ) {
                    parts.push(OpenAiContentPart::Text { text: line });
                }
                let resolved_url = media_url_resolver
                    .resolve_model_url(attachment.id.as_str(), attachment.url.as_str())
                    .await;
                if resolved_url.trim().is_empty() {
                    continue;
                }
                parts.push(OpenAiContentPart::ImageUrl {
                    image_url: OpenAiImageUrl {
                        url: resolved_url,
                        detail: Some("auto".to_string()),
                    },
                });
            } else if !attachment.mime_type.starts_with("image/") {
                parts.push(OpenAiContentPart::Text {
                    text: format_document_context(image_ref_index + 1, attachment),
                });
            }
        }

        if parts.iter().any(|part| {
            matches!(part, OpenAiContentPart::ImageUrl { .. })
                || matches!(
                    part,
                    OpenAiContentPart::Text { text }
                    if text.starts_with("uploaded_document_")
                )
        })
        {
            return OpenAiMessageContent::Parts(parts);
        }
    }

    OpenAiMessageContent::Text(flatten_current_user_input(prompt, attachments))
}

fn flatten_text_content(message: &OutboundMessage) -> String {
    let mut parts = Vec::new();
    let mut image_ref_index = 0usize;
    for part in &message.content {
        match part {
            OutboundContentPart::Text { text } => parts.push(text.clone()),
            OutboundContentPart::ImageUrl { url, media_id } => {
                image_ref_index += 1;
                parts.extend(format_image_reference_lines(
                    image_ref_index,
                    media_id.as_deref(),
                    Some(url.as_str()),
                ));
                parts.push(text_only_image_placeholder().to_string())
            }
            OutboundContentPart::Document {
                name,
                mime_type,
                size_bytes,
                ..
            } => {
                parts.push(format!(
                    "[User uploaded document: {name} ({mime_type}, {size_bytes} bytes)]"
                ));
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

    for (index, attachment) in attachments.iter().enumerate() {
        if attachment.mime_type.starts_with("image/") {
            parts.extend(format_image_reference_lines(
                index + 1,
                Some(attachment.id.as_str()),
                Some(attachment.url.as_str()),
            ));
            parts.push(text_only_image_placeholder().to_string());
        } else {
            parts.push(format_document_context(index + 1, attachment));
        }
    }

    parts.join("\n")
}

fn format_document_context(index: usize, attachment: &crate::TurnAttachment) -> String {
    let extracted_text = attachment.extracted_text.as_deref().unwrap_or("").trim();
    if extracted_text.is_empty() {
        return format!(
            "[User uploaded document {index}: {} ({}), but no readable text could be extracted.]",
            attachment.name, attachment.mime_type
        );
    }

    format!(
        "uploaded_document_{index}: {}\nMIME: {}\nContent:\n{}",
        attachment.name, attachment.mime_type, extracted_text
    )
}

fn format_image_reference_lines(
    index: usize,
    media_id: Option<&str>,
    url: Option<&str>,
) -> Vec<String> {
    let mut lines = Vec::new();

    if let Some(media_id) = media_id.map(str::trim).filter(|value| !value.is_empty()) {
        lines.push(format!("input_image_ref_{index}: {media_id}"));
    }

    if let Some(url) = url.map(str::trim).filter(|value| !value.is_empty()) {
        lines.push(format!("input_image_url_{index}: {url}"));
    }

    lines
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

fn should_retry_transport_error(error: &reqwest::Error) -> bool {
    if error.is_timeout() || error.is_connect() || error.is_request() {
        let text = error.to_string().to_ascii_lowercase();
        return text.contains("unexpected eof")
            || text.contains("tls handshake eof")
            || text.contains("connection reset")
            || text.contains("connection aborted")
            || text.contains("http2")
            || text.contains("broken pipe")
            || text.contains("timed out");
    }

    false
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
        build_current_user_message_content, build_openai_message_content, build_openai_messages,
        model_supports_image_inputs, OpenAiChatStreamChunk, OpenAiMessageContent,
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

    #[test]
    fn stream_delta_content_accepts_text_part_arrays() {
        let chunk: OpenAiChatStreamChunk = serde_json::from_value(serde_json::json!({
            "choices": [{
                "delta": {
                    "content": [
                        { "type": "text", "text": "地方性法规由" },
                        { "type": "text", "text": "本级人民代表大会及其常务委员会制定。" }
                    ]
                }
            }]
        }))
        .expect("array content chunks should parse");

        assert_eq!(
            chunk.choices[0].delta.content.as_deref(),
            Some("地方性法规由本级人民代表大会及其常务委员会制定。")
        );
    }

    #[test]
    fn stream_delta_content_accepts_nested_text_objects() {
        let chunk: OpenAiChatStreamChunk = serde_json::from_value(serde_json::json!({
            "choices": [{
                "delta": {
                    "content": { "output_text": "不是地方方法主权，而是地方主动性。" },
                    "reasoning_content": { "text": "检查概念边界" }
                }
            }]
        }))
        .expect("object content chunks should parse");

        assert_eq!(
            chunk.choices[0].delta.content.as_deref(),
            Some("不是地方方法主权，而是地方主动性。")
        );
        assert_eq!(
            chunk.choices[0].delta.reasoning_content.as_deref(),
            Some("检查概念边界")
        );
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

        let content = build_openai_message_content(&message, None, false, true, &NoopMediaResolver)
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
                media_id: Some("media/image-1.png".into()),
            }],
            tool_calls: Vec::new(),
            tool_call_id: None,
        };

        let content = build_openai_message_content(&message, None, true, true, &NoopMediaResolver)
            .await
            .expect("image content should remain available");

        match content {
            OpenAiMessageContent::Parts(parts) => {
                assert!(parts.iter().any(|part| matches!(
                    part,
                    super::OpenAiContentPart::Text { text }
                    if text == "input_image_ref_1: media/image-1.png"
                )));
                assert!(parts.iter().any(|part| matches!(
                    part,
                    super::OpenAiContentPart::Text { text }
                    if text == "input_image_url_1: https://example.com/image.png"
                )));
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
    async fn tool_history_is_text_only_even_for_multimodal_models() {
        let message = OutboundMessage {
            role: "tool".into(),
            item_id: "tool_result_1".into(),
            turn_id: "turn_1".into(),
            content: vec![OutboundContentPart::ToolResult(crate::OutboundToolResult {
                tool_call_id: "call_1".into(),
                tool_name: "image_generation".into(),
                tool_display_name: Some("GPT Image 2".into()),
                status: "completed".into(),
                arguments_text: Some("{\"prompt\":\"cat\"}".into()),
                result: serde_json::json!({
                    "kind": "tool_result",
                    "output": { "count": 1 }
                }),
                media: vec![crate::MediaAsset {
                    kind: "image".into(),
                    url: "https://example.com/generated.png".into(),
                    object_key: None,
                    mime_type: "image/png".into(),
                    size_bytes: 123,
                }],
            })],
            tool_calls: Vec::new(),
            tool_call_id: Some("call_1".into()),
        };

        let content = build_openai_message_content(&message, None, true, false, &NoopMediaResolver)
            .await
            .expect("tool history should remain available as text");

        match content {
            OpenAiMessageContent::Text(text) => {
                assert!(text.contains("[Tool Result: GPT Image 2]"));
                assert!(text.contains("image_attachment: 1 image(s) available"));
                assert!(text.contains("input_image_url_1: https://example.com/generated.png"));
            }
            OpenAiMessageContent::Parts(_) => {
                panic!("tool history should not include multimodal image parts");
            }
        }
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

    #[tokio::test]
    async fn current_user_attachments_include_reusable_input_refs() {
        let content = build_current_user_message_content(
            "edit this image",
            &[crate::TurnAttachment {
                id: "upload_1".into(),
                url: "https://example.com/upload.png".into(),
                name: "upload.png".into(),
                mime_type: "image/png".into(),
                size_bytes: 128,
                kind: Some("image".into()),
                extracted_text: None,
            }],
            true,
            &NoopMediaResolver,
        )
        .await;

        match content {
            OpenAiMessageContent::Parts(parts) => {
                assert!(parts.iter().any(|part| matches!(
                    part,
                    super::OpenAiContentPart::Text { text }
                    if text == "input_image_ref_1: upload_1"
                )));
                assert!(parts.iter().any(|part| matches!(
                    part,
                    super::OpenAiContentPart::Text { text }
                    if text == "input_image_url_1: https://example.com/upload.png"
                )));
                assert!(parts
                    .iter()
                    .any(|part| matches!(part, super::OpenAiContentPart::ImageUrl { .. })));
            }
            OpenAiMessageContent::Text(_) => {
                panic!("current user attachments should stay multimodal");
            }
        }
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
    #[serde(default, deserialize_with = "deserialize_optional_stream_text")]
    content: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_stream_text")]
    reasoning_content: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_stream_text")]
    reasoning: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAiChatStreamToolCallDelta>>,
}

fn deserialize_optional_stream_text<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(value.as_ref().and_then(extract_stream_text))
}

fn extract_stream_text(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Array(parts) => {
            let text = parts
                .iter()
                .filter_map(extract_stream_text)
                .collect::<Vec<_>>()
                .join("");

            (!text.is_empty()).then_some(text)
        }
        serde_json::Value::Object(object) => object
            .get("text")
            .or_else(|| object.get("content"))
            .or_else(|| object.get("output_text"))
            .and_then(extract_stream_text),
        _ => None,
    }
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
