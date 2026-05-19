use serde::Serialize;

#[derive(Serialize)]
pub struct ErrorResponseDto {
    pub message: String,
}
