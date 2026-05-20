use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{
    http::sessions::{
        RenameSessionDto, SessionDetailDto, SessionHistoryPageDto, SessionHistoryQueryDto,
        SessionListItemDto, SessionMessageDto, SessionToolCallSummaryDto,
    },
    security::extractors::CurrentUser,
    state::AppState,
};

fn session_access_denied_response(error: openchat_security_core::AuthorizationError) -> axum::response::Response {
    let status = if error.message == "Session not found" {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::FORBIDDEN
    };

    (status, Json(serde_json::json!({ "message": error.message }))).into_response()
}

pub async fn list_sessions(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
) -> impl IntoResponse {
    match state.chat_service.list_sessions(auth.user_id()).await {
        Ok(items) => {
            let sessions = items
                .into_iter()
                .map(|session| SessionListItemDto {
                    id: session.id,
                    title: session.title,
                    status: session.status,
                    created_at: session.created_at,
                    updated_at: session.updated_at,
                })
                .collect::<Vec<_>>();
            (StatusCode::OK, Json(sessions)).into_response()
        }
        Err(error) => (
            StatusCode::from_u16(error.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(serde_json::json!({ "message": error.message })),
        )
            .into_response(),
    }
}

pub async fn get_session(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(session_id): Path<String>,
    Query(query): Query<SessionHistoryQueryDto>,
) -> impl IntoResponse {
    if let Err(error) = state
        .resource_access
        .authorize_session(&auth, openchat_security_core::Action::Read, session_id.as_str())
        .await
    {
        return session_access_denied_response(error);
    }

    let session = match state
        .chat_service
        .get_session(auth.user_id(), session_id.as_str())
        .await
    {
        Ok(item) => item,
        Err(error) => {
            return (
                StatusCode::from_u16(error.status_code)
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(serde_json::json!({ "message": error.message })),
            )
                .into_response()
        }
    };

    let Some(session) = session else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "message": "Session not found" })),
        )
            .into_response();
    };

    match state
        .chat_service
        .session_messages_snapshot(
            auth.user_id(),
            session_id.as_str(),
            query.before_turn_id.as_deref(),
        )
        .await
    {
        Ok(snapshot) => (
            StatusCode::OK,
            Json(SessionDetailDto {
                session: SessionListItemDto {
                    id: session.id,
                    title: session.title,
                    status: session.status,
                    created_at: session.created_at,
                    updated_at: session.updated_at,
                },
                messages: snapshot
                    .messages
                    .into_iter()
                    .map(|message| SessionMessageDto {
                        id: message.id,
                        role: message.role,
                        turn_id: message.turn_id,
                        status: message.status,
                        created_at: message.created_at,
                        updated_at: message.updated_at,
                        content: message.content,
                        tool_calls: message
                            .tool_calls
                            .unwrap_or_default()
                            .into_iter()
                            .map(|tool_call| SessionToolCallSummaryDto {
                                id: tool_call.id,
                                name: tool_call.name,
                                display_name: tool_call.display_name,
                                parent_item_id: tool_call.parent_item_id,
                                arguments_text: tool_call.arguments_text,
                                result: tool_call.result,
                                status: tool_call.status.unwrap_or_else(|| "completed".to_string()),
                                media: tool_call.media.unwrap_or_default(),
                            })
                            .collect(),
                    })
                    .collect(),
                history_page: SessionHistoryPageDto {
                    has_more: snapshot.has_more,
                    next_before_turn_id: snapshot.next_before_turn_id,
                },
            }),
        )
            .into_response(),
        Err(error) => (
            StatusCode::from_u16(error.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(serde_json::json!({ "message": error.message })),
        )
            .into_response(),
    }
}

pub async fn delete_session(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    if let Err(error) = state
        .resource_access
        .authorize_session(&auth, openchat_security_core::Action::Delete, session_id.as_str())
        .await
    {
        return session_access_denied_response(error);
    }

    match state
        .chat_service
        .delete_session(auth.user_id(), session_id.as_str())
        .await
    {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "message": "Session not found" })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::from_u16(error.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(serde_json::json!({ "message": error.message })),
        )
            .into_response(),
    }
}

pub async fn rename_session(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(session_id): Path<String>,
    Json(payload): Json<RenameSessionDto>,
) -> impl IntoResponse {
    if let Err(error) = state
        .resource_access
        .authorize_session(&auth, openchat_security_core::Action::Update, session_id.as_str())
        .await
    {
        return session_access_denied_response(error);
    }

    let title = payload.title.trim();
    if title.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "message": "Session title cannot be empty" })),
        )
            .into_response();
    }

    match state
        .chat_service
        .rename_session(auth.user_id(), session_id.as_str(), title)
        .await
    {
        Ok(Some(session)) => (
            StatusCode::OK,
            Json(SessionListItemDto {
                id: session.id,
                title: session.title,
                status: session.status,
                created_at: session.created_at,
                updated_at: session.updated_at,
            }),
        )
            .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "message": "Session not found" })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::from_u16(error.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(serde_json::json!({ "message": error.message })),
        )
            .into_response(),
    }
}
