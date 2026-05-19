use std::convert::Infallible;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use futures_util::Stream;
use openchat_core::events::ChatEventEnvelope;

use crate::{
    handlers::auth::require_auth_user,
    http::{
        chat::{ChatAcceptedResponseDto, ChatRequestDto, SelectedTextModelDto, SelectedToolDto},
        errors::ErrorResponseDto,
    },
    state::AppState,
};

async fn validate_text_model_selection(
    state: &AppState,
    user_id: &str,
    selected_text_model: &SelectedTextModelDto,
) -> Result<(), ErrorResponseDto> {
    state
        .account_service
        .resolve_text_access(
            user_id,
            &openchat_core::TurnModelRef {
                runtime_provider: selected_text_model
                    .runtime_provider
                    .clone()
                    .unwrap_or_else(|| "openai_compatible".to_string()),
                model_config_id: selected_text_model.model_config_id.clone(),
                model_name: selected_text_model.model.clone().unwrap_or_default(),
                display_name: selected_text_model.display_name.clone().unwrap_or_default(),
                provider: selected_text_model.provider.clone().unwrap_or_default(),
                source: selected_text_model
                    .source
                    .clone()
                    .unwrap_or_else(|| "openchat".to_string()),
                input_modalities: selected_text_model
                    .input_modalities
                    .clone()
                    .unwrap_or_default(),
            },
        )
        .await
        .map(|_| ())
        .map_err(|error| ErrorResponseDto {
            message: error.message,
        })
}

async fn validate_tool_selection(
    state: &AppState,
    user_id: &str,
    selected_tool: &SelectedToolDto,
) -> Result<(), ErrorResponseDto> {
    state
        .tool_access_service
        .authorize_turn_tool(
            user_id,
            &openchat_core::TurnToolRef {
                runtime_provider: selected_tool
                    .runtime_provider
                    .clone()
                    .unwrap_or_else(|| "openai_compatible".to_string()),
                model_config_id: selected_tool.model_config_id.clone(),
                model_name: selected_tool.model.clone().unwrap_or_default(),
                id: selected_tool.id.clone(),
                display_name: selected_tool.display_name.clone().unwrap_or_default(),
                provider: selected_tool.provider.clone().unwrap_or_default(),
                source: selected_tool
                    .source
                    .clone()
                    .unwrap_or_else(|| "openchat".to_string()),
                tool_type: selected_tool.tool_type.clone(),
            },
        )
        .await
        .map(|_| ())
        .map_err(|error| ErrorResponseDto {
            message: error.message,
        })
}

pub async fn send_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ChatRequestDto>,
) -> impl IntoResponse {
    let user = match require_auth_user(&state, &headers).await {
        Ok(user) => user,
        Err(response) => return response,
    };

    if let Some(selected_text_model) = payload.text_model.as_ref() {
        if let Err(error) =
            validate_text_model_selection(&state, user.id.as_str(), selected_text_model).await
        {
            return (StatusCode::BAD_REQUEST, Json(error)).into_response();
        }
    }

    if let Some(selected_tools) = payload.tool_list.as_ref() {
        for selected_tool in selected_tools {
            if let Err(error) =
                validate_tool_selection(&state, user.id.as_str(), selected_tool).await
            {
                return (StatusCode::BAD_REQUEST, Json(error)).into_response();
            }
        }
    }

    match state
        .chat_service
        .start_turn(payload.into_chat_request(user.id))
        .await
    {
        Ok(accepted) => (
            StatusCode::OK,
            Json(ChatAcceptedResponseDto::from(accepted)),
        )
            .into_response(),
        Err(error) => (
            StatusCode::from_u16(error.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(ErrorResponseDto {
                message: error.message,
            }),
        )
            .into_response(),
    }
}

pub async fn stream(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut receiver = state.chat_service.subscribe(session_id.as_str()).await;

    let event_stream = async_stream::stream! {
        loop {
            match receiver.recv().await {
                Ok(payload) => {
                    let envelope = ChatEventEnvelope::new(payload);
                    yield Ok(Event::default().event("stream_event").data(envelope.data));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Sse::new(event_stream).keep_alive(KeepAlive::default())
}
