#[derive(Clone)]
pub struct ChatServiceError {
    pub status_code: u16,
    pub message: String,
}

impl ChatServiceError {
    pub fn new(status_code: u16, message: impl Into<String>) -> Self {
        Self {
            status_code,
            message: message.into(),
        }
    }
}
