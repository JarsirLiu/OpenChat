use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use crate::{
    http::{errors::ErrorResponseDto, upload::UploadedImageDto},
    security::extractors::CurrentUser,
    state::AppState,
};

const SUPPORTED_IMAGE_MIME_TYPES: &[&str] = &["image/png", "image/jpeg", "image/webp"];

pub async fn upload_images(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut uploaded = Vec::new();
    let mut index = 0usize;

    while let Some(field) = match multipart.next_field().await {
        Ok(field) => field,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponseDto {
                    message: format!("Failed to read upload payload: {error}"),
                }),
            )
                .into_response()
        }
    } {
        let file_name = field.file_name().unwrap_or("image.png").to_string();
        let mime_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();

        if !SUPPORTED_IMAGE_MIME_TYPES
            .iter()
            .any(|supported| mime_type.eq_ignore_ascii_case(supported))
        {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponseDto {
                    message: format!(
                        "Unsupported upload type `{mime_type}`. Only PNG, JPG/JPEG, and WebP are supported."
                    ),
                }),
            )
                .into_response();
        }

        let bytes = match field.bytes().await {
            Ok(bytes) => bytes.to_vec(),
            Err(error) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponseDto {
                        message: format!("Failed to read uploaded image bytes: {error}"),
                    }),
                )
                    .into_response()
            }
        };

        let object_key = build_upload_key(auth.user_id(), index, file_name.as_str());
        let stored = match state
            .media_store
            .put_owned_bytes(
                object_key.as_str(),
                bytes,
                mime_type.as_str(),
                auth.user_id(),
                None,
            )
            .await
        {
            Ok(stored) => stored,
            Err(error) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponseDto {
                        message: error.message,
                    }),
                )
                    .into_response()
            }
        };

        uploaded.push(UploadedImageDto {
            id: stored.key,
            url: stored.browser_url,
            name: file_name,
            mime_type: stored.content_type,
            size_bytes: stored.size_bytes,
        });
        index += 1;
    }

    (StatusCode::OK, Json(uploaded)).into_response()
}

fn build_upload_key(user_id: &str, index: usize, file_name: &str) -> String {
    let timestamp = current_millis();
    let sanitized_name = file_name
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_' => ch,
            _ => '_',
        })
        .collect::<String>();

    format!("uploads/users/{user_id}/{timestamp}_{index}_{sanitized_name}")
}

fn current_millis() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
