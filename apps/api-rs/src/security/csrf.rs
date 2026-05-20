use axum::{
    body::Body,
    extract::State,
    http::{header, Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::cookie::CookieJar;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;

use crate::{http::errors::ErrorResponseDto, state::AppState};

pub fn new_token() -> String {
    let mut bytes = [0_u8; 24];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn should_enforce(request: &Request<Body>) -> bool {
    !matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
    )
}

pub fn is_exempt_path(path: &str) -> bool {
    path == "/health"
}

pub fn validate_request(request: &Request<Body>, state: &AppState) -> Result<(), Response> {
    if !should_enforce(request) || is_exempt_path(request.uri().path()) {
        return Ok(());
    }

    if request.headers().contains_key(header::AUTHORIZATION) {
        return Ok(());
    }

    let cookie_jar = CookieJar::from_headers(request.headers());
    let csrf_cookie = cookie_jar
        .get(state.auth_cookies.csrf_cookie_name())
        .map(|cookie| cookie.value().to_string());
    let csrf_header = request
        .headers()
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    match (csrf_cookie, csrf_header) {
        (Some(cookie), Some(header_value)) if cookie == header_value => Ok(()),
        _ => Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponseDto {
                message: "CSRF validation failed".to_string(),
            }),
        )
            .into_response()),
    }
}

pub async fn require_csrf(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if let Err(response) = validate_request(&request, &state) {
        return response;
    }

    next.run(request).await
}
