use serde_json::Value;

use crate::MediaAsset;

#[derive(Default)]
pub(crate) struct LoopStepResult {
    pub assistant_text: String,
    pub reasoning_text: String,
    pub reasoning_started: bool,
    pub completed_tool_calls: Vec<CompletedToolCall>,
}

pub(crate) struct CompletedToolCall {
    pub tool_call_id: String,
    pub tool_name: String,
    pub tool_display_name: String,
    pub arguments_text: String,
    pub result: Value,
    pub media: Vec<MediaAsset>,
    pub failed: bool,
}

#[derive(Default)]
pub(crate) struct InProgressToolCall {
    pub tool_name: String,
    pub arguments_text: String,
}
