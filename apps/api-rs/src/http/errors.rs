use std::{collections::HashMap, sync::OnceLock};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use openchat_core::ChatServiceError;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponseDto {
    pub code: String,
    pub message: String,
    pub category: String,
    pub retryable: bool,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ErrorCodeSpec {
    code: String,
    category: String,
    retryable: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ErrorCodeCatalog {
    api_errors: HashMap<String, ErrorCodeSpec>,
}

pub const AUTHENTICATION_REQUIRED: &str = "authenticationRequired";
pub const AUTHORIZATION_DENIED: &str = "authorizationDenied";
pub const CSRF_VALIDATION_FAILED: &str = "csrfValidationFailed";
pub const SESSION_NOT_FOUND: &str = "sessionNotFound";
pub const TURN_NOT_FOUND: &str = "turnNotFound";
pub const MEDIA_NOT_FOUND: &str = "mediaNotFound";
pub const CUSTOM_MODEL_NOT_FOUND: &str = "customModelNotFound";
pub const VALIDATION_ERROR: &str = "validationError";
pub const UNSUPPORTED_UPLOAD_TYPE: &str = "unsupportedUploadType";
pub const UPLOAD_PAYLOAD_INVALID: &str = "uploadPayloadInvalid";
pub const ATTACHMENT_ACCESS_DENIED: &str = "attachmentAccessDenied";
pub const INTERNAL_ERROR: &str = "internalError";

static API_ERROR_SPECS: OnceLock<HashMap<String, ErrorCodeSpec>> = OnceLock::new();

impl ErrorResponseDto {
    pub fn from_code(code_key: &str, message: impl Into<String>) -> Self {
        let spec = spec_for_key(code_key);
        Self {
            code: spec.code.clone(),
            message: message.into(),
            category: spec.category.clone(),
            retryable: spec.retryable,
        }
    }

    pub fn from_chat_service_error(error: ChatServiceError) -> Self {
        Self::from_code(error.code, error.message)
    }
}

fn api_error_specs() -> &'static HashMap<String, ErrorCodeSpec> {
    API_ERROR_SPECS.get_or_init(|| {
        let raw = include_str!("../../../../config/error-codes.json");
        let catalog: ErrorCodeCatalog =
            serde_json::from_str(raw).expect("invalid shared error code catalog");
        catalog.api_errors
    })
}

fn spec_for_key(key: &str) -> &'static ErrorCodeSpec {
    api_error_specs()
        .get(key)
        .unwrap_or_else(|| panic!("missing api error code spec: {key}"))
}

pub fn error_response(status: StatusCode, code_key: &str, message: impl Into<String>) -> Response {
    (status, Json(ErrorResponseDto::from_code(code_key, message))).into_response()
}

pub fn chat_service_error_response_from_error(error: ChatServiceError) -> Response {
    let status_code =
        StatusCode::from_u16(error.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (
        status_code,
        Json(ErrorResponseDto::from_chat_service_error(error)),
    )
        .into_response()
}
