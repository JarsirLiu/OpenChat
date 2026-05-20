use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, Json};
use axum_extra::extract::cookie::CookieJar;
use openchat_account_core::{
    AuthError, AuthUser, CreateUserCustomModelDto, LoginRequestDto, RegisterRequestDto,
    UpsertUserProviderApiKeyDto, UserCustomModelDto, UserInfoDto, UserProviderApiKeyDto,
    UserProviderApiKeySecretDto,
};

use crate::{
    http::errors::{
        chat_service_error_response_from_error, error_response, ErrorResponseDto,
        AUTHENTICATION_REQUIRED, CUSTOM_MODEL_NOT_FOUND,
    },
    security::csrf,
    security::extractors::CurrentUser,
    state::AppState,
};

pub async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
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
        Ok(session) => {
            let user = session.user.clone();
            auth_success_response(StatusCode::CREATED, jar, &state, user, Some(session))
        }
        Err(error) => auth_error_response(error),
    }
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<LoginRequestDto>,
) -> impl IntoResponse {
    match state
        .account_service
        .login(payload.account.trim(), payload.password.as_str())
        .await
    {
        Ok(session) => {
            let user = session.user.clone();
            auth_success_response(StatusCode::OK, jar, &state, user, Some(session))
        }
        Err(error) => auth_error_response(error),
    }
}

pub async fn csrf_token(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let token = jar
        .get(state.auth_cookies.csrf_cookie_name())
        .map(|cookie| cookie.value().to_string())
        .unwrap_or_else(csrf::new_token);

    let jar = jar.add(state.auth_cookies.build_csrf_cookie(token.as_str()));
    (
        StatusCode::OK,
        jar,
        Json(serde_json::json!({ "csrf_token": token })),
    )
        .into_response()
}

pub async fn me(CurrentUser(auth): CurrentUser) -> impl IntoResponse {
    let user = AuthUser {
        id: auth.user_id().to_string(),
        username: auth.subject().username.clone().unwrap_or_default(),
        email: auth.subject().email.clone().unwrap_or_default(),
        is_admin: auth.is_admin(),
    };

    (StatusCode::OK, Json(UserInfoDto::from(user))).into_response()
}

pub async fn refresh(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let Some(refresh_token) = jar
        .get(state.auth_cookies.refresh_cookie_name())
        .map(|cookie| cookie.value().to_string())
    else {
        return auth_error_response(AuthError::new(401, "Authentication required"));
    };

    match state.account_service.refresh(refresh_token).await {
        Ok(session) => {
            let user = session.user.clone();
            auth_success_response(StatusCode::OK, jar, &state, user, Some(session))
        }
        Err(error) => auth_error_response(error),
    }
}

pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    if let Some(refresh_token) = jar
        .get(state.auth_cookies.refresh_cookie_name())
        .map(|cookie| cookie.value().to_string())
    {
        state.account_service.logout(refresh_token.as_str()).await;
    }

    let jar = jar
        .remove(state.auth_cookies.clear_access_cookie())
        .remove(state.auth_cookies.clear_refresh_cookie())
        .remove(state.auth_cookies.clear_csrf_cookie());
    (
        StatusCode::OK,
        jar,
        Json(serde_json::json!({ "status": "ok" })),
    )
        .into_response()
}

pub async fn list_user_provider_api_keys(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
) -> impl IntoResponse {
    match state
        .account_service
        .list_user_provider_api_keys(auth.user_id())
        .await
    {
        Ok(items) => Json(
            items
                .into_iter()
                .map(UserProviderApiKeyDto::from)
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => chat_service_error_response_from_error(error),
    }
}

pub async fn upsert_user_provider_api_key(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Json(payload): Json<UpsertUserProviderApiKeyDto>,
) -> impl IntoResponse {
    match state
        .account_service
        .upsert_user_provider_api_key(auth.user_id(), payload.into())
        .await
    {
        Ok(setting) => Json(UserProviderApiKeyDto::from(setting)).into_response(),
        Err(error) => chat_service_error_response_from_error(error),
    }
}

pub async fn get_user_provider_api_key(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(provider_key): Path<String>,
) -> impl IntoResponse {
    match state
        .account_service
        .get_user_provider_api_key(auth.user_id(), provider_key.as_str())
        .await
    {
        Ok(setting) => Json(UserProviderApiKeySecretDto::from(setting)).into_response(),
        Err(error) => chat_service_error_response_from_error(error),
    }
}

pub async fn list_user_custom_models(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
) -> impl IntoResponse {
    match state
        .account_service
        .list_user_custom_models(auth.user_id())
        .await
    {
        Ok(items) => Json(
            items
                .into_iter()
                .map(UserCustomModelDto::from)
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => chat_service_error_response_from_error(error),
    }
}

pub async fn create_user_custom_model(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Json(payload): Json<CreateUserCustomModelDto>,
) -> impl IntoResponse {
    match state
        .account_service
        .create_user_custom_model(auth.user_id(), payload.into())
        .await
    {
        Ok(item) => (StatusCode::CREATED, Json(UserCustomModelDto::from(item))).into_response(),
        Err(error) => chat_service_error_response_from_error(error),
    }
}

pub async fn delete_user_custom_model(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    axum::extract::Path(model_config_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    match state
        .account_service
        .delete_user_custom_model(auth.user_id(), model_config_id.as_str())
        .await
    {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => error_response(
            StatusCode::NOT_FOUND,
            CUSTOM_MODEL_NOT_FOUND,
            "Custom model not found",
        ),
        Err(error) => chat_service_error_response_from_error(error),
    }
}

fn auth_error_response(error: AuthError) -> axum::response::Response {
    let status_code =
        StatusCode::from_u16(error.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (
        status_code,
        Json(ErrorResponseDto::from_code(
            AUTHENTICATION_REQUIRED,
            error.message,
        )),
    )
        .into_response()
}

fn auth_success_response(
    status: StatusCode,
    jar: CookieJar,
    state: &AppState,
    user: AuthUser,
    session: Option<openchat_account_core::AuthSession>,
) -> axum::response::Response {
    let jar = if let Some(session) = session.as_ref() {
        let [access_cookie, refresh_cookie] = state.auth_cookies.session_cookies(session);
        jar.add(access_cookie).add(refresh_cookie)
    } else {
        jar
    };

    (status, jar, Json(UserInfoDto::from(user))).into_response()
}
