use serde_json::Value;

use crate::{MediaAsset, OutboundContentPart, OutboundToolResult, TurnAttachment};

pub fn assistant_text_to_content_json(text: &str) -> Value {
    let mut parts = Vec::new();
    if !text.is_empty() {
        parts.push(text_part(text));
    }
    Value::Array(parts)
}

pub fn user_content_to_json(text: &str, attachments: &[TurnAttachment]) -> Value {
    let mut parts = Vec::new();

    if !text.trim().is_empty() {
        parts.push(text_part(text));
    }

    for attachment in attachments {
        if attachment.mime_type.starts_with("image/") {
            parts.push(image_part(
                attachment.url.clone(),
                attachment.name.as_str(),
                Some(attachment.id.clone()),
            ));
        } else {
            parts.push(document_part(attachment));
        }
    }

    Value::Array(parts)
}

pub fn user_content_to_outbound_parts(
    text: &str,
    attachments: &[TurnAttachment],
) -> Vec<OutboundContentPart> {
    let mut parts = Vec::new();
    if !text.trim().is_empty() {
        parts.push(OutboundContentPart::Text {
            text: text.to_string(),
        });
    }
    for attachment in attachments {
        if attachment.mime_type.starts_with("image/") {
            parts.push(OutboundContentPart::ImageUrl {
                url: attachment.url.clone(),
                media_id: Some(attachment.id.clone()),
            });
        } else {
            parts.push(OutboundContentPart::Document {
                url: attachment.url.clone(),
                media_id: Some(attachment.id.clone()),
                name: attachment.name.clone(),
                mime_type: attachment.mime_type.clone(),
                size_bytes: attachment.size_bytes,
            });
        }
    }
    parts
}

pub fn value_to_outbound_content_parts(value: Value) -> Vec<OutboundContentPart> {
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
                    media_id: part
                        .get("media_id")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                }),
                "document" => Some(OutboundContentPart::Document {
                    url: part.get("url")?.as_str()?.to_string(),
                    media_id: part
                        .get("media_id")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    name: part
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("document")
                        .to_string(),
                    mime_type: part
                        .get("mime_type")
                        .and_then(Value::as_str)
                        .unwrap_or("application/octet-stream")
                        .to_string(),
                    size_bytes: part
                        .get("size_bytes")
                        .and_then(Value::as_u64)
                        .and_then(|value| usize::try_from(value).ok())
                        .unwrap_or_default(),
                }),
                "tool_result" => Some(OutboundContentPart::ToolResult(OutboundToolResult {
                    tool_call_id: part.get("toolCallId")?.as_str()?.to_string(),
                    tool_name: part.get("toolName")?.as_str()?.to_string(),
                    tool_display_name: part
                        .get("toolDisplayName")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    status: part
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("completed")
                        .to_string(),
                    arguments_text: part
                        .get("argumentsText")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    result: part
                        .get("result")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!({})),
                    media: part
                        .get("media")
                        .and_then(Value::as_array)
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(parse_media_asset)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default(),
                })),
                _ => None,
            }
        })
        .collect()
}

pub fn append_image_media_parts(parts: &mut Vec<OutboundContentPart>, media: &[crate::MediaAsset]) {
    for item in media.iter().filter(|media| media.kind == "image") {
        if item.url.trim().is_empty() {
            continue;
        }
        parts.push(OutboundContentPart::ImageUrl {
            url: item.url.clone(),
            media_id: item.object_key.clone(),
        });
    }
}

pub fn tool_result_to_content_json(result: &OutboundToolResult) -> Value {
    serde_json::json!([{
        "type": "tool_result",
        "toolCallId": result.tool_call_id,
        "toolName": result.tool_name,
        "toolDisplayName": result.tool_display_name,
        "status": result.status,
        "argumentsText": result.arguments_text,
        "result": result.result,
        "media": result.media,
    }])
}

fn parse_media_asset(value: &Value) -> Option<MediaAsset> {
    serde_json::from_value::<MediaAsset>(value.clone()).ok()
}

fn text_part(text: &str) -> Value {
    serde_json::json!({
        "type": "text",
        "text": text,
    })
}

fn image_part(url: String, alt: &str, media_id: Option<String>) -> Value {
    serde_json::json!({
        "type": "image",
        "url": url,
        "alt": alt,
        "media_id": media_id,
    })
}

fn document_part(attachment: &TurnAttachment) -> Value {
    serde_json::json!({
        "type": "document",
        "url": attachment.url,
        "name": attachment.name,
        "mime_type": attachment.mime_type,
        "size_bytes": attachment.size_bytes,
        "media_id": attachment.id,
    })
}

#[cfg(test)]
mod tests {
    use super::{tool_result_to_content_json, value_to_outbound_content_parts};
    use crate::{MediaAsset, OutboundContentPart, OutboundToolResult};
    use serde_json::json;

    #[test]
    fn tool_result_content_round_trips_as_a_structured_part() {
        let tool_result = OutboundToolResult {
            tool_call_id: "call_1".into(),
            tool_name: "image_generation".into(),
            tool_display_name: Some("Image Generation".into()),
            status: "completed".into(),
            arguments_text: Some("{\"prompt\":\"cat\"}".into()),
            result: json!({
                "kind": "tool_result",
                "output": { "count": 1 }
            }),
            media: vec![MediaAsset {
                kind: "image".into(),
                url: "https://example.com/image.png".into(),
                object_key: Some("media/object-1.png".into()),
                mime_type: "image/png".into(),
                size_bytes: 123,
            }],
        };

        let content = tool_result_to_content_json(&tool_result);
        let parts = value_to_outbound_content_parts(content);

        assert_eq!(parts.len(), 1);
        assert!(matches!(
            &parts[0],
            OutboundContentPart::ToolResult(parsed)
                if parsed.tool_call_id == "call_1"
                    && parsed.tool_name == "image_generation"
                    && parsed.media.len() == 1
        ));
    }
}
