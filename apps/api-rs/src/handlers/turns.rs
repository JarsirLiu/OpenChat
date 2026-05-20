use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{
    http::errors::{
        chat_service_error_response, error_response, AUTHORIZATION_DENIED, SESSION_NOT_FOUND,
        TURN_NOT_FOUND,
    },
    http::turns::TurnInterruptAcceptedDto,
    security::extractors::CurrentUser,
    state::AppState,
};

pub async fn interrupt_turn(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path((session_id, turn_id)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Err(error) = state
        .resource_access
        .authorize_session(&auth, openchat_security_core::Action::Update, session_id.as_str())
        .await
    {
        if error.message == "Session not found" {
            return error_response(StatusCode::NOT_FOUND, SESSION_NOT_FOUND, error.message);
        }

        return error_response(StatusCode::FORBIDDEN, AUTHORIZATION_DENIED, error.message);
    }

    match state
        .chat_service
        .interrupt_turn(auth.user_id(), session_id.as_str(), turn_id.as_str())
        .await
    {
        Ok(true) => (
            StatusCode::ACCEPTED,
            Json(TurnInterruptAcceptedDto { ok: true }),
        )
            .into_response(),
        Ok(false) => error_response(
            StatusCode::NOT_FOUND,
            TURN_NOT_FOUND,
            "Turn not found or no longer running",
        ),
        Err(error) => chat_service_error_response(error.status_code, error.message),
    }
}
