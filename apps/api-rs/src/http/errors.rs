use std::{collections::HashMap, sync::OnceLock};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
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
pub const PROVIDER_API_KEY_REQUIRED: &str = "providerApiKeyRequired";
pub const MODEL_UNAVAILABLE: &str = "modelUnavailable";
pub const TOOL_UNAVAILABLE: &str = "toolUnavailable";
pub const RESOURCE_NOT_FOUND: &str = "resourceNotFound";
pub const CONFLICT: &str = "conflict";
pub const RATE_LIMITED: &str = "rateLimited";
pub const UPSTREAM_ERROR: &str = "upstreamError";
pub const SERVICE_UNAVAILABLE: &str = "serviceUnavailable";
pub const GATEWAY_TIMEOUT: &str = "gatewayTimeout";
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

pub fn error_response(
    status: StatusCode,
    code_key: &str,
    message: impl Into<String>,
) -> Response {
    (status, Json(ErrorResponseDto::from_code(code_key, message))).into_response()
}

pub fn code_key_for_status(status: u16) -> &'static str {
    match status {
        400 => VALIDATION_ERROR,
        401 => AUTHENTICATION_REQUIRED,
        403 => AUTHORIZATION_DENIED,
        404 => RESOURCE_NOT_FOUND,
        409 => CONFLICT,
        429 => RATE_LIMITED,
        502 => UPSTREAM_ERROR,
        503 => SERVICE_UNAVAILABLE,
        504 => GATEWAY_TIMEOUT,
        _ => INTERNAL_ERROR,
    }
}

pub fn code_key_for_message(status: u16, message: &str) -> &'static str {
    if message == "Session not found" {
        return SESSION_NOT_FOUND;
    }

    if message == "Turn not found or no longer running" {
        return TURN_NOT_FOUND;
    }

    if message == "Custom model not found" {
        return CUSTOM_MODEL_NOT_FOUND;
    }

    if message == "Media not found" {
        return MEDIA_NOT_FOUND;
    }

    if message == "CSRF validation failed" {
        return CSRF_VALIDATION_FAILED;
    }

    if message == "One or more uploaded images do not belong to the current user" {
        return ATTACHMENT_ACCESS_DENIED;
    }

    if message == "Session title cannot be empty" {
        return VALIDATION_ERROR;
    }

    if message.contains("API Key") || message.contains("api key") {
        return PROVIDER_API_KEY_REQUIRED;
    }

    if message.contains("Selected custom model is not available")
        || message.contains("Selected custom image model is not available")
        || message.contains("Selected text model is not available")
        || message.contains("Selected image tool model is not available")
    {
        return MODEL_UNAVAILABLE;
    }

    if message.contains("The selected tool is not available") || message.contains("not enabled")
    {
        return TOOL_UNAVAILABLE;
    }

    code_key_for_status(status)
}

pub fn chat_service_error_response(status: u16, message: impl Into<String>) -> Response {
    let message = message.into();
    let status_code = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let code_key = code_key_for_message(status, message.as_str());
    error_response(status_code, code_key, message)
}
