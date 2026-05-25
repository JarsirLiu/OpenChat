use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct UploadedAttachmentDto {
    pub id: String,
    pub url: String,
    pub name: String,
    pub mime_type: String,
    pub size_bytes: usize,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extracted_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_error: Option<String>,
}
