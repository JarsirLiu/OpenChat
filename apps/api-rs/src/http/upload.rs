use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct UploadedImageDto {
    pub id: String,
    pub url: String,
    pub name: String,
    pub mime_type: String,
    pub size_bytes: usize,
}
