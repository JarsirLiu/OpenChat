#[derive(Clone)]
pub enum OutboundContentPart {
    Text { text: String },
    ImageUrl { url: String },
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
