use crate::protocol::TerminalReasonDto;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurnTerminalReasonCode {
    UserRequested,
    SessionRecovered,
    ModelConnectTimeout,
    ModelStreamIdleTimeout,
    TranscriptProjectionFailed,
    ProviderAuthenticationFailed,
    UpstreamError,
    RuntimeError,
}

impl TurnTerminalReasonCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UserRequested => "user_requested",
            Self::SessionRecovered => "session_recovered",
            Self::ModelConnectTimeout => "model_connect_timeout",
            Self::ModelStreamIdleTimeout => "model_stream_idle_timeout",
            Self::TranscriptProjectionFailed => "transcript_projection_failed",
            Self::ProviderAuthenticationFailed => "provider_authentication_failed",
            Self::UpstreamError => "upstream_error",
            Self::RuntimeError => "runtime_error",
        }
    }
}

#[derive(Clone, Debug)]
pub struct TurnTerminalReason {
    code: TurnTerminalReasonCode,
    message: String,
}

impl TurnTerminalReason {
    pub fn new(code: TurnTerminalReasonCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn user_requested() -> Self {
        Self::new(TurnTerminalReasonCode::UserRequested, "用户已停止本轮回复")
    }

    pub fn session_recovered() -> Self {
        Self::new(
            TurnTerminalReasonCode::SessionRecovered,
            "服务已恢复，上一轮响应已中止",
        )
    }

    pub fn model_connect_timeout() -> Self {
        Self::new(
            TurnTerminalReasonCode::ModelConnectTimeout,
            "模型连接超时，请重试或切换模型",
        )
    }

    pub fn model_stream_idle_timeout() -> Self {
        Self::new(
            TurnTerminalReasonCode::ModelStreamIdleTimeout,
            "模型响应超时，请重试或切换模型",
        )
    }

    pub fn transcript_projection_failed(message: impl Into<String>) -> Self {
        Self::new(TurnTerminalReasonCode::TranscriptProjectionFailed, message)
    }

    pub fn upstream_error(message: impl Into<String>) -> Self {
        Self::new(TurnTerminalReasonCode::UpstreamError, message)
    }

    pub fn runtime_error(message: impl Into<String>) -> Self {
        Self::new(TurnTerminalReasonCode::RuntimeError, message)
    }

    pub fn from_chat_service_error(error: &crate::ChatServiceError) -> Self {
        match error.code {
            crate::PROVIDER_AUTHENTICATION_FAILED => Self::new(
                TurnTerminalReasonCode::ProviderAuthenticationFailed,
                error.message.clone(),
            ),
            _ => Self::upstream_error(error.message.clone()),
        }
    }

    pub fn code(&self) -> TurnTerminalReasonCode {
        self.code
    }

    pub fn code_str(&self) -> &'static str {
        self.code.as_str()
    }

    pub fn message(&self) -> &str {
        self.message.as_str()
    }

    pub fn to_event_dto(&self) -> TerminalReasonDto {
        TerminalReasonDto {
            code: self.code_str().to_string(),
            message: Some(self.message.clone()),
        }
    }
}
