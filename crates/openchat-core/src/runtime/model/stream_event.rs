use std::pin::Pin;

use futures_util::Stream;
use serde_json::Value;

use crate::ChatServiceError;

pub type ModelEventStream =
    Pin<Box<dyn Stream<Item = Result<ModelStreamEvent, ChatServiceError>> + Send>>;

#[derive(Clone, Debug)]
pub enum ModelStreamEvent {
    TextDelta(String),
    ReasoningDelta(String),
    ToolCallStart {
        tool_call_id: String,
        tool_name: String,
        arguments: Option<Value>,
    },
    ToolCallArgumentsDelta {
        tool_call_id: String,
        delta: String,
    },
    ToolCallComplete {
        tool_call_id: String,
        tool_name: String,
        arguments_text: String,
    },
}
