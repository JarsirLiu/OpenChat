use crate::OutboundMessage;

#[derive(Clone)]
pub struct TurnModelRef {
    pub runtime_provider: String,
    pub model_config_id: String,
    pub model_name: String,
    pub display_name: String,
    pub provider: String,
    pub source: String,
    pub input_modalities: Vec<String>,
}

#[derive(Clone)]
pub struct TurnToolRef {
    pub runtime_provider: String,
    pub model_config_id: String,
    pub model_name: String,
    pub id: String,
    pub display_name: String,
    pub provider: String,
    pub source: String,
    pub tool_type: String,
}

#[derive(Clone)]
pub struct TurnAttachment {
    pub id: String,
    pub url: String,
    pub name: String,
    pub mime_type: String,
    pub size_bytes: usize,
}

#[derive(Clone)]
pub struct TurnPlan {
    pub user_id: String,
    pub session_id: String,
    pub prompt: String,
    pub attachments: Vec<TurnAttachment>,
    pub history: Vec<OutboundMessage>,
    pub text_model: TurnModelRef,
    pub tool_list: Vec<TurnToolRef>,
}
