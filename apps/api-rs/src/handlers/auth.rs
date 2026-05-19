use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use openchat_account_core::{
    AuthError, AuthResponseDto, AuthUser, CreateUserCustomModelDto, LoginRequestDto,
    LogoutRequestDto, RefreshRequestDto, RegisterRequestDto, UpsertUserProviderSettingDto,
    UserCustomModelDto, UserInfoDto, UserProviderSettingDto,
};

use crate::{http::errors::ErrorResponseDto, state::AppState};

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequestDto>,
) -> impl IntoResponse {
    match state
        .account_service
        .register(
            payload.email.trim(),
            payload.password.as_str(),
            payload.username.as_deref(),
        )
        .await
    {
        Ok(session) => (StatusCode::CREATED, Json(AuthResponseDto::from(session))).into_response(),
        Err(error) => auth_error_response(error),
    }
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequestDto>,
) -> impl IntoResponse {
    match state
        .account_service
        .login(payload.account.trim(), payload.password.as_str())
        .await
    {
        Ok(session) => (StatusCode::OK, Json(AuthResponseDto::from(session))).into_response(),
        Err(error) => auth_error_response(error),
    }
}

pub async fn me(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let Some(token) = bearer_token(&headers) else {
        return auth_error_response(AuthError::new(401, "Authentication required"));
    };

    match state.account_service.current_user(token).await {
        Ok(user) => (StatusCode::OK, Json(UserInfoDto::from(user))).into_response(),
        Err(error) => auth_error_response(error),
    }
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequestDto>,
) -> impl IntoResponse {
    match state.account_service.refresh(payload.refresh_token).await {
        Ok(session) => (StatusCode::OK, Json(AuthResponseDto::from(session))).into_response(),
        Err(error) => auth_error_response(error),
    }
}

pub async fn logout(
    State(state): State<AppState>,
    Json(payload): Json<LogoutRequestDto>,
) -> impl IntoResponse {
    state
        .account_service
        .logout(payload.refresh_token.as_str())
        .await;
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))).into_response()
}

pub async fn list_user_provider_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match require_auth_user(&state, &headers).await {
        Ok(user) => user,
        Err(response) => return response,
    };

    match state
        .account_service
        .list_user_provider_settings(user.id.as_str())
        .await
    {
        Ok(items) => Json(
            items
                .into_iter()
                .map(UserProviderSettingDto::from)
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => (
            StatusCode::from_u16(error.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(serde_json::json!({ "message": error.message })),
        )
            .into_response(),
    }
}

pub async fn upsert_user_provider_setting(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpsertUserProviderSettingDto>,
) -> impl IntoResponse {
    let user = match require_auth_user(&state, &headers).await {
        Ok(user) => user,
        Err(response) => return response,
    };

    match state
        .account_service
        .upsert_user_provider_setting(user.id.as_str(), payload.into())
        .await
    {
        Ok(setting) => Json(UserProviderSettingDto::from(setting)).into_response(),
        Err(error) => (
            StatusCode::from_u16(error.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(serde_json::json!({ "message": error.message })),
        )
            .into_response(),
    }
}

pub async fn list_user_custom_models(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match require_auth_user(&state, &headers).await {
        Ok(user) => user,
        Err(response) => return response,
    };

    match state
        .account_service
        .list_user_custom_models(user.id.as_str())
        .await
    {
        Ok(items) => Json(
            items
                .into_iter()
                .map(UserCustomModelDto::from)
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => (
            StatusCode::from_u16(error.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(serde_json::json!({ "message": error.message })),
        )
            .into_response(),
    }
}

pub async fn create_user_custom_model(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateUserCustomModelDto>,
) -> impl IntoResponse {
    let user = match require_auth_user(&state, &headers).await {
        Ok(user) => user,
        Err(response) => return response,
    };

    match state
        .account_service
        .create_user_custom_model(user.id.as_str(), payload.into())
        .await
    {
        Ok(item) => (StatusCode::CREATED, Json(UserCustomModelDto::from(item))).into_response(),
        Err(error) => (
            StatusCode::from_u16(error.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(serde_json::json!({ "message": error.message })),
        )
            .into_response(),
    }
}

pub async fn delete_user_custom_model(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(model_config_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let user = match require_auth_user(&state, &headers).await {
        Ok(user) => user,
        Err(response) => return response,
    };

    match state
        .account_service
        .delete_user_custom_model(user.id.as_str(), model_config_id.as_str())
        .await
    {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "message": "Custom model not found" })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::from_u16(error.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(serde_json::json!({ "message": error.message })),
        )
            .into_response(),
    }
}

pub async fn require_auth_user(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthUser, axum::response::Response> {
    let Some(token) = bearer_token(headers) else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponseDto {
                message: "Authentication required".to_string(),
            }),
        )
            .into_response());
    };

    state
        .account_service
        .current_user(token)
        .await
        .map_err(|error| {
            (
                StatusCode::from_u16(error.status_code)
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(ErrorResponseDto {
                    message: error.message,
                }),
            )
                .into_response()
        })
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let header_value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    header_value.strip_prefix("Bearer ")
}

fn auth_error_response(error: AuthError) -> axum::response::Response {
    let status_code =
        StatusCode::from_u16(error.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (
        status_code,
        Json(ErrorResponseDto {
            message: error.message,
        }),
    )
        .into_response()
}
