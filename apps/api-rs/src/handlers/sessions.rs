use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{
    http::errors::{
        chat_service_error_response_from_error, error_response, AUTHORIZATION_DENIED,
        SESSION_NOT_FOUND, VALIDATION_ERROR,
    },
    http::sessions::{
        RenameSessionDto, SessionDetailDto, SessionHistoryPageDto, SessionHistoryQueryDto,
        SessionListItemDto, SessionThreadItemDto, SessionThreadItemImageDto, SessionTurnDto,
    },
    security::extractors::CurrentUser,
    state::AppState,
    time::format_millis_timestamp,
};

fn session_access_denied_response(
    error: openchat_security_core::AuthorizationError,
) -> axum::response::Response {
    if error.message == "Session not found" {
        return error_response(StatusCode::NOT_FOUND, SESSION_NOT_FOUND, error.message);
    }

    error_response(StatusCode::FORBIDDEN, AUTHORIZATION_DENIED, error.message)
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
                    transcript_version: Some(session.transcript_version),
                    transcript_migration_status: Some(session.transcript_migration_status),
                    transcript_migration_error: session.transcript_migration_error,
                    created_at: format_millis_timestamp(session.created_at.as_str()),
                    updated_at: format_millis_timestamp(session.updated_at.as_str()),
                })
                .collect::<Vec<_>>();
            (StatusCode::OK, Json(sessions)).into_response()
        }
        Err(error) => chat_service_error_response_from_error(error),
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
        .authorize_session(
            &auth,
            openchat_security_core::Action::Read,
            session_id.as_str(),
        )
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
        Err(error) => return chat_service_error_response_from_error(error),
    };

    let Some(session) = session else {
        return error_response(
            StatusCode::NOT_FOUND,
            SESSION_NOT_FOUND,
            "Session not found",
        );
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
                    transcript_version: Some(session.transcript_version),
                    transcript_migration_status: Some(session.transcript_migration_status),
                    transcript_migration_error: session.transcript_migration_error,
                    created_at: format_millis_timestamp(session.created_at.as_str()),
                    updated_at: format_millis_timestamp(session.updated_at.as_str()),
                },
                turns: snapshot
                    .turns
                    .into_iter()
                    .map(|turn| SessionTurnDto {
                        id: turn.id,
                        session_id: turn.session_id,
                        status: turn.status,
                        started_at: turn
                            .started_at
                            .map(|value| format_millis_timestamp(value.as_str())),
                        completed_at: turn
                            .completed_at
                            .map(|value| format_millis_timestamp(value.as_str())),
                        terminal_reason: turn.terminal_reason,
                        items: turn
                            .items
                            .into_iter()
                            .map(|item| SessionThreadItemDto {
                                id: item.id,
                                item_type: item.item_type,
                                session_id: item.session_id,
                                turn_id: item.turn_id,
                                status: item.status,
                                seq: item.seq,
                                created_at: item
                                    .created_at
                                    .map(|value| format_millis_timestamp(value.as_str())),
                                updated_at: item
                                    .updated_at
                                    .map(|value| format_millis_timestamp(value.as_str())),
                                parent_id: item.parent_id,
                                content: item.content,
                                text: item.text,
                                prompt: item.prompt,
                                revised_prompt: item.revised_prompt,
                                model: item.model,
                                size: item.size,
                                quality: item.quality,
                                count: item.count,
                                source_tool_call_id: item.source_tool_call_id,
                                source_tool_name: item.source_tool_name,
                                images: item
                                    .images
                                    .into_iter()
                                    .map(|image| SessionThreadItemImageDto {
                                        url: image.url,
                                        object_key: image.object_key,
                                        mime_type: image.mime_type,
                                        size_bytes: image.size_bytes,
                                    })
                                    .collect(),
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
        Err(error) => chat_service_error_response_from_error(error),
    }
}

pub async fn delete_session(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    if let Err(error) = state
        .resource_access
        .authorize_session(
            &auth,
            openchat_security_core::Action::Delete,
            session_id.as_str(),
        )
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
        Ok(false) => error_response(
            StatusCode::NOT_FOUND,
            SESSION_NOT_FOUND,
            "Session not found",
        ),
        Err(error) => chat_service_error_response_from_error(error),
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
        .authorize_session(
            &auth,
            openchat_security_core::Action::Update,
            session_id.as_str(),
        )
        .await
    {
        return session_access_denied_response(error);
    }

    let title = payload.title.trim();
    if title.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            VALIDATION_ERROR,
            "Session title cannot be empty",
        );
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
                transcript_version: Some(session.transcript_version),
                transcript_migration_status: Some(session.transcript_migration_status),
                transcript_migration_error: session.transcript_migration_error,
                created_at: format_millis_timestamp(session.created_at.as_str()),
                updated_at: format_millis_timestamp(session.updated_at.as_str()),
            }),
        )
            .into_response(),
        Ok(None) => error_response(
            StatusCode::NOT_FOUND,
            SESSION_NOT_FOUND,
            "Session not found",
        ),
        Err(error) => chat_service_error_response_from_error(error),
    }
}
