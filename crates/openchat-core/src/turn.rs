#[derive(Clone)]
pub struct TurnAccepted {
    pub status: String,
    pub session_id: String,
    pub turn_id: String,
}

impl TurnAccepted {
    pub fn new(session_id: String, turn_id: String) -> Self {
        Self {
            status: "done".into(),
            session_id,
            turn_id,
        }
    }
}

#[derive(Clone)]
pub struct TurnContext {
    pub session_id: String,
    pub turn_id: String,
    pub prompt: String,
}

impl TurnContext {
    pub fn new(session_id: String, turn_id: String, prompt: String) -> Self {
        Self {
            session_id,
            turn_id,
            prompt,
        }
    }
}
