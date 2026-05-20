use openchat_infra::stores::{PersistedSessionMessage, PersistedSessionToolCall};

use super::{
    collect_attached_tool_calls,
    types::{OutboundContentPart, OutboundMessage, OutboundToolCall, OutboundToolResult},
    value_to_outbound_content_parts,
};

pub fn normalize_session_history(
    messages: Vec<PersistedSessionMessage>,
    tool_calls: Vec<PersistedSessionToolCall>,
) -> Vec<OutboundMessage> {
    messages
        .into_iter()
        .map(|message| {
            let attached_tool_calls = if message.role == "assistant" {
                collect_attached_tool_calls(message.id.as_str(), message.turn_id.as_str(), &tool_calls)
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

fn normalize_content_parts(
    content: serde_json::Value,
    attached_tool_calls: Vec<PersistedSessionToolCall>,
) -> Vec<OutboundContentPart> {
    let mut parts = value_to_outbound_content_parts(content);

    for tool_call in attached_tool_calls {
        parts.push(OutboundContentPart::ToolResult(OutboundToolResult {
            tool_call_id: tool_call.id.clone(),
            tool_name: tool_call.tool_name.clone(),
            tool_display_name: tool_call.tool_display_name.clone(),
            status: tool_call.status.clone(),
            arguments_text: tool_call.arguments_text.clone(),
            result: tool_call
                .result_json
                .as_deref()
                .and_then(|raw| serde_json::from_str(raw).ok())
                .unwrap_or_else(|| serde_json::json!({})),
            media: crate::parse_media_assets_json(tool_call.media_json.as_deref()),
        }));
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

#[cfg(test)]
mod tests {
    use openchat_infra::stores::{PersistedSessionMessage, PersistedSessionToolCall};
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
                "[{\"kind\":\"image\",\"url\":\"https://example.com/image.png\",\"objectKey\":\"media/object-1.png\",\"mimeType\":\"image/png\",\"sizeBytes\":123},{\"kind\":\"image\",\"url\":\"https://example.com/image-2.png\",\"objectKey\":\"media/object-2.png\",\"mimeType\":\"image/png\",\"sizeBytes\":456}]"
                    .to_string(),
            ),
        }];

        let history = normalize_session_history(messages, tool_calls);

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].content.len(), 2);
        assert!(matches!(
            &history[0].content[1],
            super::OutboundContentPart::ToolResult(_)
        ));
    }

    #[test]
    fn tool_result_keeps_raw_image_url_out_of_text() {
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
                "[{\"kind\":\"image\",\"url\":\"https://example.com/image.png\",\"objectKey\":\"media/object-1.png\",\"mimeType\":\"image/png\",\"sizeBytes\":123}]"
                    .to_string(),
            ),
        }];

        let history = normalize_session_history(messages, tool_calls);
        let tool_result = match &history[0].content[1] {
            super::OutboundContentPart::ToolResult(tool_result) => tool_result,
            _ => panic!("expected tool result text"),
        };

        assert_eq!(tool_result.media.len(), 1);
        assert_eq!(tool_result.media[0].url, "https://example.com/image.png");
    }
}
