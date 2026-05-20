use crate::{
    user_content_to_outbound_parts, OutboundContentPart, OutboundMessage, OutboundToolCall,
    OutboundToolResult, TurnAttachment,
};

use super::loop_step_result::{CompletedToolCall, LoopStepResult};

pub(crate) struct HistoryAssembler;

impl HistoryAssembler {
    pub fn append_step_result(
        rolling_history: &mut Vec<OutboundMessage>,
        included_user_message_in_history: &mut bool,
        user_item_id: &str,
        turn_id: &str,
        prompt: &str,
        attachments: &[TurnAttachment],
        step_index: usize,
        step_result: &LoopStepResult,
    ) {
        if !*included_user_message_in_history {
            rolling_history.push(OutboundMessage {
                role: "user".into(),
                item_id: user_item_id.to_string(),
                turn_id: turn_id.to_string(),
                content: user_content_to_outbound_parts(prompt, attachments),
                tool_calls: Vec::new(),
                tool_call_id: None,
            });
            *included_user_message_in_history = true;
        }

        rolling_history.push(OutboundMessage {
            role: "assistant".into(),
            item_id: format!("assistant_step_{step_index}"),
            turn_id: turn_id.to_string(),
            content: if step_result.assistant_text.trim().is_empty() {
                Vec::new()
            } else {
                vec![OutboundContentPart::Text {
                    text: step_result.assistant_text.clone(),
                }]
            },
            tool_calls: step_result
                .completed_tool_calls
                .iter()
                .map(|call| OutboundToolCall {
                    id: call.tool_call_id.clone(),
                    name: call.tool_name.clone(),
                    arguments_text: call.arguments_text.clone(),
                })
                .collect(),
            tool_call_id: None,
        });

        for tool_call in &step_result.completed_tool_calls {
            rolling_history.push(OutboundMessage {
                role: "tool".into(),
                item_id: format!("tool_result_{}", tool_call.tool_call_id),
                turn_id: turn_id.to_string(),
                content: tool_result_to_outbound_parts(tool_call),
                tool_calls: Vec::new(),
                tool_call_id: Some(tool_call.tool_call_id.clone()),
            });
        }
    }
}

fn tool_result_to_outbound_parts(call: &CompletedToolCall) -> Vec<OutboundContentPart> {
    vec![OutboundContentPart::ToolResult(OutboundToolResult {
        tool_call_id: call.tool_call_id.clone(),
        tool_name: call.tool_name.clone(),
        tool_display_name: Some(call.tool_display_name.clone()),
        status: if call.failed {
            "failed".into()
        } else {
            "completed".into()
        },
        arguments_text: Some(call.arguments_text.clone()),
        result: call.result.clone(),
        media: call.media.clone(),
    })]
}

#[cfg(test)]
mod tests {
    use super::{tool_result_to_outbound_parts, CompletedToolCall};
    use crate::{MediaAsset, OutboundContentPart, OutboundToolResult};
    use serde_json::json;

    fn sample_completed_tool_call(image_urls: &[&str]) -> CompletedToolCall {
        let media = image_urls
            .iter()
            .map(|url| MediaAsset {
                kind: "image".into(),
                url: (*url).to_string(),
                object_key: None,
                mime_type: "image/png".into(),
                size_bytes: 128,
            })
            .collect::<Vec<_>>();
        CompletedToolCall {
            tool_call_id: "call_1".into(),
            tool_name: "image_generation".into(),
            tool_display_name: "Image Generation".into(),
            arguments_text: "{\"prompt\":\"cat\"}".into(),
            result: json!({
                "kind": "tool_result",
                "output": {
                    "count": media.len(),
                }
            }),
            media,
            failed: false,
        }
    }

    #[test]
    fn tool_history_parts_are_structured() {
        let parts = tool_result_to_outbound_parts(&sample_completed_tool_call(&[
            "https://example.com/generated.png",
            "https://example.com/generated-2.png",
        ]));

        assert_eq!(parts.len(), 1);
        assert!(matches!(
            &parts[0],
            OutboundContentPart::ToolResult(OutboundToolResult { tool_call_id, tool_name, .. })
                if tool_call_id == "call_1" && tool_name == "image_generation"
        ));
    }
}
