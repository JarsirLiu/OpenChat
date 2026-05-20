use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use openchat_security_core::{AuthContext, AuthenticationError};

use crate::http::errors::{ErrorResponseDto, AUTHENTICATION_REQUIRED};

#[derive(Clone, Debug)]
pub struct CurrentUser(pub AuthContext);

#[derive(Clone, Debug)]
pub struct MaybeCurrentUser(pub Option<AuthContext>);

#[derive(Clone, Debug)]
pub(crate) struct AuthenticationFailure(pub AuthenticationError);

#[async_trait]
impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(auth) = parts.extensions.get::<AuthContext>() {
            return Ok(Self(auth.clone()));
        }

        let message = parts
            .extensions
            .get::<AuthenticationFailure>()
            .map(|failure| failure.0.message.clone())
            .unwrap_or_else(|| "Authentication required".to_string());

        Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponseDto::from_code(
                AUTHENTICATION_REQUIRED,
                message,
            )),
        )
            .into_response())
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for MaybeCurrentUser
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(parts.extensions.get::<AuthContext>().cloned()))
    }
}
