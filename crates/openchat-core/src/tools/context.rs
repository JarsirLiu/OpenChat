use serde_json::Value;

use crate::{MediaAsset, OutboundMessage, TurnAttachment, TurnToolRef};

#[derive(Clone)]
pub struct ToolInvocation {
    pub user_id: String,
    pub session_id: String,
    pub turn_id: String,
    pub tool_call_id: String,
    pub arguments_text: String,
    pub current_attachments: Vec<TurnAttachment>,
    pub history: Vec<OutboundMessage>,
    pub tool: TurnToolRef,
}

#[derive(Clone)]
pub struct ToolExecutionResult {
    pub media: Vec<MediaAsset>,
    pub result: Value,
}
