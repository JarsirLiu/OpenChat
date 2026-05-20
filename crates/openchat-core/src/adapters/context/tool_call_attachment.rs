use openchat_infra::stores::PersistedSessionToolCall;

pub fn collect_attached_tool_calls(
    message_id: &str,
    turn_id: &str,
    tool_calls: &[PersistedSessionToolCall],
) -> Vec<PersistedSessionToolCall> {
    tool_calls
        .iter()
        .filter(|tool_call| {
            tool_call.parent_item_id.as_deref() == Some(message_id)
                || (tool_call.parent_item_id.is_none() && tool_call.turn_id == turn_id)
        })
        .cloned()
        .collect()
}
