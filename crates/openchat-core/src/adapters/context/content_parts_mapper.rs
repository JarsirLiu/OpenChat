use serde_json::Value;

use crate::{OutboundContentPart, TurnAttachment};

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
