pub const AUTHENTICATION_REQUIRED: &str = "authenticationRequired";
pub const AUTHORIZATION_DENIED: &str = "authorizationDenied";
pub const SESSION_NOT_FOUND: &str = "sessionNotFound";
pub const VALIDATION_ERROR: &str = "validationError";
pub const PROVIDER_API_KEY_REQUIRED: &str = "providerApiKeyRequired";
pub const PROVIDER_AUTHENTICATION_FAILED: &str = "providerAuthenticationFailed";
pub const MODEL_UNAVAILABLE: &str = "modelUnavailable";
pub const TOOL_UNAVAILABLE: &str = "toolUnavailable";
pub const RATE_LIMITED: &str = "rateLimited";
pub const UPSTREAM_ERROR: &str = "upstreamError";
pub const SERVICE_UNAVAILABLE: &str = "serviceUnavailable";
pub const GATEWAY_TIMEOUT: &str = "gatewayTimeout";
pub const INTERNAL_ERROR: &str = "internalError";

#[derive(Clone)]
pub struct ChatServiceError {
    pub code: &'static str,
    pub status_code: u16,
    pub message: String,
}

impl ChatServiceError {
    pub fn new(status_code: u16, message: impl Into<String>) -> Self {
        Self::from_code(default_code_for_status(status_code), status_code, message)
    }

    pub fn from_code(code: &'static str, status_code: u16, message: impl Into<String>) -> Self {
        Self {
            code,
            status_code,
            message: message.into(),
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::from_code(VALIDATION_ERROR, 400, message)
    }

    pub fn session_not_found(message: impl Into<String>) -> Self {
        Self::from_code(SESSION_NOT_FOUND, 404, message)
    }

    pub fn provider_api_key_required(message: impl Into<String>) -> Self {
        Self::from_code(PROVIDER_API_KEY_REQUIRED, 400, message)
    }

    pub fn provider_authentication_failed(message: impl Into<String>) -> Self {
        Self::from_code(PROVIDER_AUTHENTICATION_FAILED, 502, message)
    }

    pub fn model_unavailable(message: impl Into<String>) -> Self {
        Self::from_code(MODEL_UNAVAILABLE, 400, message)
    }

    pub fn tool_unavailable(message: impl Into<String>) -> Self {
        Self::from_code(TOOL_UNAVAILABLE, 400, message)
    }

    pub fn upstream(message: impl Into<String>) -> Self {
        Self::from_code(UPSTREAM_ERROR, 502, message)
    }
}

fn default_code_for_status(status_code: u16) -> &'static str {
    match status_code {
        400 => VALIDATION_ERROR,
        401 => AUTHENTICATION_REQUIRED,
        403 => AUTHORIZATION_DENIED,
        404 => SESSION_NOT_FOUND,
        429 => RATE_LIMITED,
        502 => UPSTREAM_ERROR,
        503 => SERVICE_UNAVAILABLE,
        504 => GATEWAY_TIMEOUT,
        _ => INTERNAL_ERROR,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ChatServiceError, PROVIDER_AUTHENTICATION_FAILED, SESSION_NOT_FOUND, VALIDATION_ERROR,
    };

    #[test]
    fn generic_constructor_assigns_default_code_from_status() {
        let error = ChatServiceError::new(400, "invalid input");
        assert_eq!(error.code, VALIDATION_ERROR);
    }

    #[test]
    fn specific_constructor_preserves_explicit_code() {
        let error = ChatServiceError::provider_authentication_failed("invalid credentials");
        assert_eq!(error.code, PROVIDER_AUTHENTICATION_FAILED);
        assert_eq!(error.status_code, 502);
    }

    #[test]
    fn session_not_found_uses_specific_not_found_code() {
        let error = ChatServiceError::session_not_found("Session not found");
        assert_eq!(error.code, SESSION_NOT_FOUND);
        assert_eq!(error.status_code, 404);
    }
}
