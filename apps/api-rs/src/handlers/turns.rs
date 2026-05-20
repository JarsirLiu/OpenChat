use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{
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
        let status = if error.message == "Session not found" {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::FORBIDDEN
        };
        return (
            status,
            Json(serde_json::json!({ "message": error.message })),
        )
            .into_response();
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
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "message": "Turn not found or no longer running" })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::from_u16(error.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(serde_json::json!({ "message": error.message })),
        )
            .into_response(),
    }
}
