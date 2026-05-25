use openchat_core::{ChatRequest, SelectedTextModel, SelectedTool, TurnAccepted};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize)]
pub struct ChatRequestDto {
    pub session_id: String,
    pub prompt: String,
    pub attachments: Option<Vec<UploadedAttachmentDto>>,
    pub text_model: Option<SelectedTextModelDto>,
    pub tool_list: Option<Vec<SelectedToolDto>>,
}

#[derive(Clone, Deserialize)]
pub struct UploadedAttachmentDto {
    pub id: String,
    pub url: String,
    pub name: String,
    pub mime_type: String,
    pub size_bytes: usize,
    pub kind: Option<String>,
    pub extracted_text: Option<String>,
    pub extraction_error: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct SelectedTextModelDto {
    pub model_config_id: String,
    pub display_name: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub runtime_provider: Option<String>,
    pub source: Option<String>,
    #[serde(rename = "type")]
    pub model_type: Option<String>,
    pub input_modalities: Option<Vec<String>>,
}

#[derive(Clone, Deserialize)]
pub struct SelectedToolDto {
    pub model_config_id: String,
    pub id: String,
    pub display_name: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub runtime_provider: Option<String>,
    pub source: Option<String>,
    #[serde(rename = "type")]
    pub tool_type: String,
}

#[derive(Serialize)]
pub struct ChatAcceptedResponseDto {
    pub status: String,
    pub session_id: String,
    pub turn_id: String,
}

impl From<ChatRequestDto> for ChatRequest {
    fn from(value: ChatRequestDto) -> Self {
        Self {
            user_id: String::new(),
            session_id: value.session_id,
            prompt: value.prompt,
            attachments: value
                .attachments
                .unwrap_or_default()
                .into_iter()
                .map(|attachment| openchat_core::UploadedAttachment {
                    id: attachment.id,
                    url: attachment.url,
                    name: attachment.name,
                    mime_type: attachment.mime_type,
                    size_bytes: attachment.size_bytes,
                    kind: attachment.kind,
                    extracted_text: attachment.extracted_text,
                })
                .collect(),
            text_model: value.text_model.map(SelectedTextModel::from),
            tool_list: value
                .tool_list
                .unwrap_or_default()
                .into_iter()
                .map(SelectedTool::from)
                .collect(),
        }
    }
}

impl ChatRequestDto {
    pub fn into_chat_request(self, user_id: String) -> ChatRequest {
        let mut request = ChatRequest::from(self);
        request.user_id = user_id;
        request
    }
}

impl From<SelectedTextModelDto> for SelectedTextModel {
    fn from(value: SelectedTextModelDto) -> Self {
        Self {
            model_config_id: value.model_config_id,
            display_name: value.display_name,
            model_name: value.model,
            provider: value.provider,
            runtime_provider: value.runtime_provider,
            source: value.source,
            model_type: value.model_type,
            input_modalities: value.input_modalities.unwrap_or_default(),
        }
    }
}

impl From<SelectedToolDto> for SelectedTool {
    fn from(value: SelectedToolDto) -> Self {
        Self {
            model_config_id: value.model_config_id,
            id: value.id,
            display_name: value.display_name,
            model_name: value.model,
            provider: value.provider,
            runtime_provider: value.runtime_provider,
            source: value.source,
            tool_type: value.tool_type,
        }
    }
}

impl From<TurnAccepted> for ChatAcceptedResponseDto {
    fn from(value: TurnAccepted) -> Self {
        Self {
            status: value.status,
            session_id: value.session_id,
            turn_id: value.turn_id,
        }
    }
}
