use super::types::{OutboundMessage, SessionContext};
use std::collections::BTreeSet;

const SESSION_HISTORY_WINDOW_SIZE: usize = 30;

pub fn session_history_window_size() -> usize {
    SESSION_HISTORY_WINDOW_SIZE
}

pub fn build_session_context(session_id: String, history: Vec<OutboundMessage>) -> SessionContext {
    let history = trim_history_window(history);
    SessionContext::new(session_id, history)
}

fn trim_history_window(history: Vec<OutboundMessage>) -> Vec<OutboundMessage> {
    let turn_order: Vec<String> = history.iter().map(|message| message.turn_id.clone()).fold(
        Vec::new(),
        |mut acc, turn_id| {
            if acc.last() != Some(&turn_id) {
                acc.push(turn_id);
            }
            acc
        },
    );

    if turn_order.len() <= SESSION_HISTORY_WINDOW_SIZE {
        return history;
    }

    let skip_count = turn_order.len() - SESSION_HISTORY_WINDOW_SIZE;
    let kept_turn_ids: BTreeSet<String> = turn_order.into_iter().skip(skip_count).collect();

    history
        .into_iter()
        .filter(|message| kept_turn_ids.contains(&message.turn_id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::build_session_context;
    use crate::OutboundMessage;

    fn message(turn_index: usize, role: &str, item_index: usize) -> OutboundMessage {
        OutboundMessage {
            role: role.to_string(),
            item_id: format!("item_{item_index}"),
            turn_id: format!("turn_{turn_index}"),
            content: Vec::new(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }
    }

    #[test]
    fn keeps_only_last_thirty_turns() {
        let history = (0..35)
            .flat_map(|turn_index| {
                [
                    message(turn_index, "user", turn_index * 10),
                    message(turn_index, "assistant", turn_index * 10 + 1),
                ]
            })
            .collect();
        let context = build_session_context("sess_1".to_string(), history);

        assert_eq!(context.history.len(), 60);
        assert_eq!(
            context.history.first().map(|item| item.turn_id.as_str()),
            Some("turn_5")
        );
        assert_eq!(
            context.history.last().map(|item| item.turn_id.as_str()),
            Some("turn_34")
        );
    }
}
