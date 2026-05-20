use crate::{
    append_image_media_parts, format_tool_result_text, user_content_to_outbound_parts,
    OutboundContentPart, OutboundMessage, OutboundToolCall, TurnAttachment,
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

fn tool_result_to_history_text(call: &CompletedToolCall) -> String {
    format_tool_result_text(
        call.tool_display_name.as_str(),
        if call.failed { "failed" } else { "completed" },
        Some(call.arguments_text.as_str()),
        Some(&call.result),
        call.media.as_slice(),
    )
}

fn tool_result_to_outbound_parts(call: &CompletedToolCall) -> Vec<OutboundContentPart> {
    let mut parts = vec![OutboundContentPart::Text {
        text: tool_result_to_history_text(call),
    }];
    append_image_media_parts(&mut parts, call.media.as_slice());

    parts
}

#[cfg(test)]
mod tests {
    use super::{tool_result_to_history_text, tool_result_to_outbound_parts, CompletedToolCall};
    use crate::{MediaAsset, OutboundContentPart};
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
    fn tool_history_parts_include_generated_image_url() {
        let parts = tool_result_to_outbound_parts(&sample_completed_tool_call(&[
            "https://example.com/generated.png",
            "https://example.com/generated-2.png",
        ]));

        assert_eq!(parts.len(), 3);
        assert!(matches!(parts[0], OutboundContentPart::Text { .. }));
        assert!(matches!(
            &parts[1],
            OutboundContentPart::ImageUrl { url, .. } if url == "https://example.com/generated.png"
        ));
        assert!(matches!(
            &parts[2],
            OutboundContentPart::ImageUrl { url, .. } if url == "https://example.com/generated-2.png"
        ));
    }

    #[test]
    fn tool_history_text_keeps_image_reference_out_of_plain_text() {
        let text = tool_result_to_history_text(&sample_completed_tool_call(&[
            "https://example.com/generated.png",
        ]));

        assert!(text.contains("image_attachment: 1 image(s) available"));
        assert!(!text.contains("https://example.com/generated.png"));
    }
}
