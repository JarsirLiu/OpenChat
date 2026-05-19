use openchat_infra::sqlite::{PersistedSessionMessage, PersistedSessionToolCall};
use serde_json::Value;

use crate::{parse_media_assets_json, MediaAsset};

use super::types::{OutboundContentPart, OutboundMessage, OutboundToolCall};

pub fn normalize_session_history(
    messages: Vec<PersistedSessionMessage>,
    tool_calls: Vec<PersistedSessionToolCall>,
) -> Vec<OutboundMessage> {
    messages
        .into_iter()
        .map(|message| {
            let attached_tool_calls = if message.role == "assistant" {
                collect_attached_tool_calls(&message, &tool_calls)
            } else {
                Vec::new()
            };
            let outbound_tool_calls = attached_tool_calls
                .iter()
                .map(tool_call_to_outbound_tool_call)
                .collect();

            OutboundMessage {
                role: message.role,
                item_id: message.id,
                turn_id: message.turn_id,
                content: normalize_content_parts(message.content, attached_tool_calls),
                tool_calls: outbound_tool_calls,
                tool_call_id: message.tool_call_id,
            }
        })
        .collect()
}

fn collect_attached_tool_calls(
    message: &PersistedSessionMessage,
    tool_calls: &[PersistedSessionToolCall],
) -> Vec<PersistedSessionToolCall> {
    tool_calls
        .iter()
        .filter(|tool_call| {
            tool_call.parent_item_id.as_deref() == Some(message.id.as_str())
                || (tool_call.parent_item_id.is_none() && tool_call.turn_id == message.turn_id)
        })
        .cloned()
        .collect()
}

fn normalize_content_parts(
    content: Value,
    attached_tool_calls: Vec<PersistedSessionToolCall>,
) -> Vec<OutboundContentPart> {
    let mut parts = value_to_outbound_content_parts(content);

    for tool_call in attached_tool_calls {
        parts.push(OutboundContentPart::Text {
            text: format_tool_result_text(&tool_call),
        });

        for media in tool_call_media(&tool_call) {
            if media.kind == "image" && !media.url.trim().is_empty() {
                parts.push(OutboundContentPart::ImageUrl { url: media.url });
            }
        }
    }

    parts
}

pub fn tool_call_to_outbound_tool_call(tool_call: &PersistedSessionToolCall) -> OutboundToolCall {
    OutboundToolCall {
        id: tool_call.id.clone(),
        name: tool_call.tool_name.clone(),
        arguments_text: tool_call.arguments_text.clone().unwrap_or_default(),
    }
}

fn value_to_outbound_content_parts(value: Value) -> Vec<OutboundContentPart> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|part| {
            let item_type = part.get("type")?.as_str()?;
            match item_type {
                "text" => Some(OutboundContentPart::Text {
                    text: part.get("text")?.as_str()?.to_string(),
                }),
                "image" => Some(OutboundContentPart::ImageUrl {
                    url: part.get("url")?.as_str()?.to_string(),
                }),
                _ => None,
            }
        })
        .collect()
}

fn format_tool_result_text(tool_call: &PersistedSessionToolCall) -> String {
    let display_name = tool_call
        .tool_display_name
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(tool_call.tool_name.as_str());

    let mut lines = vec![
        format!("[Tool Result: {display_name}]"),
        format!("status: {}", tool_call.status),
    ];

    if let Some(arguments_text) = tool_call
        .arguments_text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("arguments: {arguments_text}"));
    }

    if let Some(result_text) = tool_call
        .result_json
        .as_deref()
        .and_then(parse_result_json_text)
    {
        lines.push(format!("result: {result_text}"));
    }

    let image_count = tool_call_media(tool_call)
        .iter()
        .filter(|media| media.kind == "image")
        .count();
    if image_count > 0 {
        lines.push(format!(
            "image_attachment: {image_count} image(s) available"
        ));
    }

    lines.join("\n")
}

fn tool_call_media(tool_call: &PersistedSessionToolCall) -> Vec<MediaAsset> {
    parse_media_assets_json(tool_call.media_json.as_deref())
}

fn parse_result_json_text(raw: &str) -> Option<String> {
    let mut value = serde_json::from_str::<Value>(raw).ok()?;
    if let Some(output) = value.get_mut("output").and_then(Value::as_object_mut) {
        output.remove("downloadUrl");
    }
    if let Some(output) = value.get("output") {
        return Some(output.to_string());
    }
    Some(value.to_string())
}

#[cfg(test)]
mod tests {
    use openchat_infra::sqlite::{PersistedSessionMessage, PersistedSessionToolCall};
    use serde_json::json;

    use super::normalize_session_history;

    #[test]
    fn assistant_history_includes_tool_results() {
        let messages = vec![PersistedSessionMessage {
            id: "assistant_1".to_string(),
            session_id: "sess_1".to_string(),
            turn_id: "turn_1".to_string(),
            role: "assistant".to_string(),
            status: "completed".to_string(),
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
            content: json!([{ "type": "text", "text": "done" }]),
            tool_call_id: None,
        }];
        let tool_calls = vec![PersistedSessionToolCall {
            id: "tool_1".to_string(),
            turn_id: "turn_1".to_string(),
            parent_item_id: Some("assistant_1".to_string()),
            tool_name: "image_gen_oai".to_string(),
            tool_display_name: Some("GPT Image 2".to_string()),
            arguments_text: Some("{\"prompt\":\"cat\"}".to_string()),
            result_json: Some("{\"kind\":\"image\",\"message\":\"ok\"}".to_string()),
            status: "completed".to_string(),
            media_json: Some(
                "[{\"kind\":\"image\",\"url\":\"https://example.com/image.png\",\"mimeType\":\"image/png\",\"sizeBytes\":123},{\"kind\":\"image\",\"url\":\"https://example.com/image-2.png\",\"mimeType\":\"image/png\",\"sizeBytes\":456}]"
                    .to_string(),
            ),
        }];

        let history = normalize_session_history(messages, tool_calls);

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].content.len(), 4);
    }

    #[test]
    fn tool_result_text_does_not_inline_raw_image_url() {
        let messages = vec![PersistedSessionMessage {
            id: "assistant_1".to_string(),
            session_id: "sess_1".to_string(),
            turn_id: "turn_1".to_string(),
            role: "assistant".to_string(),
            status: "completed".to_string(),
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
            content: json!([{ "type": "text", "text": "done" }]),
            tool_call_id: None,
        }];
        let tool_calls = vec![PersistedSessionToolCall {
            id: "tool_1".to_string(),
            turn_id: "turn_1".to_string(),
            parent_item_id: Some("assistant_1".to_string()),
            tool_name: "image_gen_oai".to_string(),
            tool_display_name: Some("GPT Image 2".to_string()),
            arguments_text: None,
            result_json: None,
            status: "completed".to_string(),
            media_json: Some(
                "[{\"kind\":\"image\",\"url\":\"https://example.com/image.png\",\"mimeType\":\"image/png\",\"sizeBytes\":123}]"
                    .to_string(),
            ),
        }];

        let history = normalize_session_history(messages, tool_calls);
        let text = match &history[0].content[1] {
            super::OutboundContentPart::Text { text } => text,
            _ => panic!("expected tool result text"),
        };

        assert!(text.contains("image_attachment: 1 image(s) available"));
        assert!(!text.contains("https://example.com/image.png"));
    }
}
