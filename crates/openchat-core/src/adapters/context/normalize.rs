use openchat_infra::stores::PersistedThreadItem;

use super::{
    types::{OutboundContentPart, OutboundMessage, OutboundToolCall, OutboundToolResult},
    value_to_outbound_content_parts,
};

pub fn normalize_thread_item_history(items: Vec<PersistedThreadItem>) -> Vec<OutboundMessage> {
    items
        .iter()
        .filter(|item| {
            item.item_type == "userMessage"
                || item.item_type == "reasoning"
                || item.item_type == "agentMessage"
        })
        .flat_map(|item| {
            let attached_tool_calls = if item.item_type == "agentMessage" {
                items
                    .iter()
                    .filter(|candidate| candidate.parent_id.as_deref() == Some(item.id.as_str()))
                    .filter_map(thread_item_to_outbound_tool_call)
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };

            let primary_message = OutboundMessage {
                role: match item.item_type.as_str() {
                    "userMessage" => "user".into(),
                    "reasoning" => "reasoning".into(),
                    _ => "assistant".into(),
                },
                item_id: item.id.clone(),
                turn_id: item.turn_id.clone(),
                content: thread_item_to_outbound_parts(item),
                tool_calls: attached_tool_calls,
                tool_call_id: None,
            };

            let tool_messages = items
                .iter()
                .filter(|candidate| candidate.parent_id.as_deref() == Some(item.id.as_str()))
                .filter_map(thread_item_to_outbound_tool_message)
                .collect::<Vec<_>>();

            let mut normalized = Vec::with_capacity(1 + tool_messages.len());
            normalized.push(primary_message);
            normalized.extend(tool_messages);
            normalized
        })
        .collect()
}

fn thread_item_to_outbound_parts(item: &PersistedThreadItem) -> Vec<OutboundContentPart> {
    if let Some(content_json) = item.content_json.as_deref() {
        if let Ok(value) = serde_json::from_str(content_json) {
            let parts = value_to_outbound_content_parts(value);
            if !parts.is_empty() {
                return parts;
            }
        }
    }

    item.text
        .as_deref()
        .filter(|text| !text.trim().is_empty())
        .map(|text| {
            vec![OutboundContentPart::Text {
                text: text.to_string(),
            }]
        })
        .unwrap_or_default()
}

fn thread_item_to_outbound_tool_call(item: &PersistedThreadItem) -> Option<OutboundToolCall> {
    let tool_call_id = item.source_tool_call_id.as_ref()?;
    let tool_name = item.source_tool_name.as_ref()?;
    Some(OutboundToolCall {
        id: tool_call_id.clone(),
        name: tool_name.clone(),
        arguments_text: serde_json::json!({
            "prompt": item.prompt,
            "size": item.size,
            "quality": item.quality,
            "n": item.count,
        })
        .to_string(),
    })
}

fn thread_item_to_outbound_tool_message(item: &PersistedThreadItem) -> Option<OutboundMessage> {
    if item.item_type != "imageGeneration" {
        return None;
    }

    let tool_call_id = item
        .source_tool_call_id
        .clone()
        .unwrap_or_else(|| item.id.clone());
    let tool_name = item
        .source_tool_name
        .clone()
        .unwrap_or_else(|| "image_generation".to_string());
    let media = item
        .images_json
        .as_deref()
        .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|entry| {
            Some(crate::MediaAsset {
                kind: "image".into(),
                url: entry.get("url")?.as_str()?.to_string(),
                object_key: None,
                mime_type: entry.get("mimeType")?.as_str()?.to_string(),
                size_bytes: entry
                    .get("sizeBytes")
                    .and_then(|value| value.as_u64())
                    .and_then(|value| usize::try_from(value).ok())
                    .unwrap_or(0),
            })
        })
        .collect::<Vec<_>>();

    Some(OutboundMessage {
        role: "tool".into(),
        item_id: format!("tool_result_{}", tool_call_id),
        turn_id: item.turn_id.clone(),
        content: vec![OutboundContentPart::ToolResult(OutboundToolResult {
            tool_call_id: tool_call_id.clone(),
            tool_name: tool_name.clone(),
            tool_display_name: Some(tool_name),
            status: item.status.clone(),
            arguments_text: Some(
                serde_json::json!({
                    "prompt": item.prompt,
                    "size": item.size,
                    "quality": item.quality,
                    "n": item.count,
                })
                .to_string(),
            ),
            result: serde_json::json!({
                "kind": "tool_result",
                "output": {
                    "count": media.len(),
                    "prompt": item.prompt,
                }
            }),
            media,
        })],
        tool_calls: Vec::new(),
        tool_call_id: Some(tool_call_id),
    })
}
