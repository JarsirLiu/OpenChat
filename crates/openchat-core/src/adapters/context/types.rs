use crate::MediaAsset;
use serde_json::Value;

#[derive(Clone)]
pub struct OutboundToolResult {
    pub tool_call_id: String,
    pub tool_name: String,
    pub tool_display_name: Option<String>,
    pub status: String,
    pub arguments_text: Option<String>,
    pub result: Value,
    pub media: Vec<MediaAsset>,
}

#[derive(Clone)]
pub enum OutboundContentPart {
    Text {
        text: String,
    },
    ImageUrl {
        url: String,
        media_id: Option<String>,
    },
    Document {
        url: String,
        media_id: Option<String>,
        name: String,
        mime_type: String,
        size_bytes: usize,
    },
    ToolResult(OutboundToolResult),
}

#[derive(Clone)]
pub struct OutboundToolCall {
    pub id: String,
    pub name: String,
    pub arguments_text: String,
}

#[derive(Clone)]
pub struct OutboundMessage {
    pub role: String,
    pub item_id: String,
    pub turn_id: String,
    pub content: Vec<OutboundContentPart>,
    pub tool_calls: Vec<OutboundToolCall>,
    pub tool_call_id: Option<String>,
}

#[derive(Clone)]
pub struct SessionContext {
    pub session_id: String,
    pub history: Vec<OutboundMessage>,
}

impl SessionContext {
    pub fn new(session_id: String, history: Vec<OutboundMessage>) -> Self {
        Self {
            session_id,
            history,
        }
    }
}
