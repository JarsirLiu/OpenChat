use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderValue, Request},
    middleware::Next,
    response::Response,
};
use axum_extra::extract::cookie::CookieJar;
use openchat_security_core::{AuthContext, AuthMethod};

use crate::security::extractors::AuthenticationFailure;
use crate::state::AppState;

pub async fn resolve_auth_context(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    if let Some((token, method)) = resolve_access_token(&request, &state) {
        match state.authenticator.authenticate_access_token(&token).await {
            Ok(auth) => {
                let auth = if matches!(method, AuthMethod::SessionCookie) {
                    AuthContext::new(auth.subject().clone(), method)
                } else {
                    auth
                };
                request.extensions_mut().insert::<AuthContext>(auth);
            }
            Err(error) => {
                request
                    .extensions_mut()
                    .insert(AuthenticationFailure(error));
            }
        }
    }

    next.run(request).await
}

fn bearer_token(header_value: Option<&HeaderValue>) -> Option<&str> {
    let header_value = header_value?.to_str().ok()?;
    header_value.strip_prefix("Bearer ")
}

fn resolve_access_token(request: &Request<Body>, state: &AppState) -> Option<(String, AuthMethod)> {
    if let Some(token) = bearer_token(request.headers().get(header::AUTHORIZATION)) {
        return Some((token.to_string(), AuthMethod::AccessToken));
    }

    let cookie_jar = CookieJar::from_headers(request.headers());
    cookie_jar
        .get(state.auth_cookies.access_cookie_name())
        .map(|cookie| (cookie.value().to_string(), AuthMethod::SessionCookie))
}
