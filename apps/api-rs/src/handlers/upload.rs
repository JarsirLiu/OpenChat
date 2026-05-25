use crate::{
    http::{
        errors::{
            chat_service_error_response_from_error, ErrorResponseDto, UNSUPPORTED_UPLOAD_TYPE,
            UPLOAD_PAYLOAD_INVALID,
        },
        upload::UploadedAttachmentDto,
    },
    security::extractors::CurrentUser,
    state::AppState,
};
use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

const SUPPORTED_IMAGE_MIME_TYPES: &[&str] = &["image/png", "image/jpeg", "image/webp"];
const SUPPORTED_DOCUMENT_MIME_TYPES: &[&str] = &[
    "text/plain",
    "text/markdown",
    "application/pdf",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
];
const MAX_UPLOAD_BYTES: usize = 100 * 1024 * 1024;
const MAX_FILES_PER_UPLOAD: usize = 12;

pub async fn upload_images(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    mut multipart: Multipart,
) -> impl IntoResponse {
    upload_multipart(&state, auth.user_id(), &mut multipart, UploadMode::ImagesOnly).await
}

pub async fn upload_files(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    mut multipart: Multipart,
) -> impl IntoResponse {
    upload_multipart(&state, auth.user_id(), &mut multipart, UploadMode::ImagesAndDocuments).await
}

#[derive(Clone, Copy)]
enum UploadMode {
    ImagesOnly,
    ImagesAndDocuments,
}

async fn upload_multipart(
    state: &AppState,
    user_id: &str,
    multipart: &mut Multipart,
    mode: UploadMode,
) -> axum::response::Response {
    let mut uploaded = Vec::new();
    let mut index = 0usize;

    while let Some(field) = match multipart.next_field().await {
        Ok(field) => field,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponseDto::from_code(
                    UPLOAD_PAYLOAD_INVALID,
                    format!("上传内容读取失败，可能是文件超过服务器限制或网络中断：{error}"),
                )),
            )
                .into_response()
        }
    } {
        if index >= MAX_FILES_PER_UPLOAD {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponseDto::from_code(
                    UPLOAD_PAYLOAD_INVALID,
                    too_many_files_message(),
                )),
            )
                .into_response();
        }

        let file_name = field.file_name().unwrap_or("image.png").to_string();
        let mime_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();

        if !is_supported_upload_type(mime_type.as_str(), file_name.as_str(), mode) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponseDto::from_code(
                    UNSUPPORTED_UPLOAD_TYPE,
                    unsupported_upload_message(mode, mime_type.as_str()),
                )),
            )
                .into_response();
        }

        let bytes = match field.bytes().await {
            Ok(bytes) => bytes.to_vec(),
            Err(error) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponseDto::from_code(
                        UPLOAD_PAYLOAD_INVALID,
                        format!("文件「{file_name}」读取失败，可能是文件过大或上传中断：{error}"),
                    )),
                )
                    .into_response()
            }
        };
        if bytes.len() > MAX_UPLOAD_BYTES {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponseDto::from_code(
                    UPLOAD_PAYLOAD_INVALID,
                    file_too_large_message(file_name.as_str()),
                )),
            )
                .into_response();
        }

        let (extracted_text, extraction_error) = if matches!(mode, UploadMode::ImagesAndDocuments)
            && !mime_type.to_ascii_lowercase().starts_with("image/")
        {
            match crate::document_text::extract_supported_document_text(
                bytes.as_slice(),
                mime_type.as_str(),
                file_name.as_str(),
            ) {
                Ok(text) => (text, None),
                Err(error) => (
                    Some(crate::document_text::document_text_extraction_notice(
                        file_name.as_str(),
                        error.as_str(),
                    )),
                    Some(crate::document_text::document_text_extraction_user_message(
                        file_name.as_str(),
                    )),
                ),
            }
        } else {
            (None, None)
        };

        let object_key = build_upload_key(user_id, index, file_name.as_str());
        let stored = match state
            .media_store
            .put_owned_bytes(
                object_key.as_str(),
                bytes,
                mime_type.as_str(),
                user_id,
                None,
                None,
            )
            .await
        {
            Ok(stored) => stored,
            Err(error) => return chat_service_error_response_from_error(error),
        };

        uploaded.push(UploadedAttachmentDto {
            id: stored.key,
            url: stored.browser_url,
            name: file_name,
            mime_type: stored.content_type,
            size_bytes: stored.size_bytes,
            kind: if mime_type.to_ascii_lowercase().starts_with("image/") {
                "image".to_string()
            } else {
                "document".to_string()
            },
            extracted_text,
            extraction_error,
        });
        index += 1;
    }

    (StatusCode::OK, Json(uploaded)).into_response()
}

fn is_supported_upload_type(mime_type: &str, file_name: &str, mode: UploadMode) -> bool {
    let normalized_mime = mime_type.to_ascii_lowercase();
    if SUPPORTED_IMAGE_MIME_TYPES
        .iter()
        .any(|supported| normalized_mime == *supported)
    {
        return true;
    }

    if !matches!(mode, UploadMode::ImagesAndDocuments) {
        return false;
    }

    let lower_name = file_name.to_ascii_lowercase();
    SUPPORTED_DOCUMENT_MIME_TYPES
        .iter()
        .any(|supported| normalized_mime == *supported)
        || lower_name.ends_with(".txt")
        || lower_name.ends_with(".md")
        || lower_name.ends_with(".markdown")
        || lower_name.ends_with(".pdf")
        || lower_name.ends_with(".docx")
}

fn unsupported_upload_message(mode: UploadMode, mime_type: &str) -> String {
    match mode {
        UploadMode::ImagesOnly => format!(
            "不支持的上传类型「{mime_type}」。当前支持 PNG、JPG/JPEG、WebP 图片。"
        ),
        UploadMode::ImagesAndDocuments => format!(
            "不支持的上传类型「{mime_type}」。当前支持 PNG、JPG/JPEG、WebP 图片，以及 TXT、Markdown、PDF、DOCX 文档。"
        ),
    }
}

fn upload_limit_mb() -> usize {
    MAX_UPLOAD_BYTES / 1024 / 1024
}

fn too_many_files_message() -> String {
    format!("一次最多上传 {MAX_FILES_PER_UPLOAD} 个文件，请分批上传")
}

fn file_too_large_message(file_name: &str) -> String {
    format!(
        "文件「{file_name}」过大，单个文件请控制在 {} MB 以内；文档内容会自动截断到模型上下文范围",
        upload_limit_mb()
    )
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
