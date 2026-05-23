#[derive(Clone)]
pub struct SelectedTextModel {
    pub model_config_id: String,
    pub display_name: Option<String>,
    pub model_name: Option<String>,
    pub provider: Option<String>,
    pub runtime_provider: Option<String>,
    pub source: Option<String>,
    pub model_type: Option<String>,
    pub input_modalities: Vec<String>,
}

#[derive(Clone)]
pub struct UploadedAttachment {
    pub id: String,
    pub url: String,
    pub name: String,
    pub mime_type: String,
    pub size_bytes: usize,
    pub kind: Option<String>,
    pub extracted_text: Option<String>,
}

#[derive(Clone)]
pub struct SelectedTool {
    pub model_config_id: String,
    pub id: String,
    pub display_name: Option<String>,
    pub model_name: Option<String>,
    pub provider: Option<String>,
    pub runtime_provider: Option<String>,
    pub source: Option<String>,
    pub tool_type: String,
}

#[derive(Clone)]
pub struct ChatRequest {
    pub user_id: String,
    pub session_id: String,
    pub prompt: String,
    pub attachments: Vec<UploadedAttachment>,
    pub text_model: Option<SelectedTextModel>,
    pub tool_list: Vec<SelectedTool>,
}
