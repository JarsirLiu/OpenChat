use std::{error::Error, fmt::{Display, Formatter}};

use crate::{Action, ResourceKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthenticationErrorKind {
    MissingCredentials,
    InvalidCredentials,
    ExpiredCredentials,
    Internal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthenticationError {
    pub kind: AuthenticationErrorKind,
    pub message: String,
}

impl AuthenticationError {
    pub fn new(kind: AuthenticationErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn missing_credentials() -> Self {
        Self::new(
            AuthenticationErrorKind::MissingCredentials,
            "Authentication required",
        )
    }

    pub fn invalid_credentials(message: impl Into<String>) -> Self {
        Self::new(AuthenticationErrorKind::InvalidCredentials, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(AuthenticationErrorKind::Internal, message)
    }
}

impl Display for AuthenticationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message.as_str())
    }
}

impl Error for AuthenticationError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizationError {
    pub message: String,
}

impl AuthorizationError {
    pub fn forbidden(action: Action, resource_kind: ResourceKind) -> Self {
        Self {
            message: format!("Not allowed to {} {}", action.as_str(), resource_kind.as_str()),
        }
    }
}

impl Display for AuthorizationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message.as_str())
    }
}

impl Error for AuthorizationError {}
