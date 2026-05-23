use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use openchat_core::protocol::ChatEventEnvelope;

use crate::{
    document_text::{truncate_document_text, MAX_TOTAL_DOCUMENT_CONTEXT_CHARS},
    http::{
        chat::{ChatAcceptedResponseDto, ChatRequestDto, SelectedTextModelDto, SelectedToolDto},
        errors::{
            chat_service_error_response_from_error, error_response, ErrorResponseDto,
            ATTACHMENT_ACCESS_DENIED, AUTHORIZATION_DENIED, SESSION_NOT_FOUND, VALIDATION_ERROR,
        },
    },
    security::extractors::CurrentUser,
    state::AppState,
};

const MAX_ATTACHMENTS_PER_TURN: usize = 8;

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
        .map_err(ErrorResponseDto::from_chat_service_error)
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
                image_defaults: None,
            },
        )
        .await
        .map(|_| ())
        .map_err(ErrorResponseDto::from_chat_service_error)
}

fn validate_selected_tool_list(selected_tools: &[SelectedToolDto]) -> Result<(), ErrorResponseDto> {
    let image_tool_count = selected_tools
        .iter()
        .filter(|tool| tool.tool_type == "image")
        .count();

    if image_tool_count > 1 {
        return Err(ErrorResponseDto::from_code(
            VALIDATION_ERROR,
            "Only one image generation model can be selected for a turn",
        ));
    }

    Ok(())
}

async fn normalize_attachments(
    state: &AppState,
    user_id: &str,
    payload: &mut ChatRequestDto,
) -> Result<(), ErrorResponseDto> {
    let Some(attachments) = payload.attachments.as_mut() else {
        return Ok(());
    };

    if attachments.len() > MAX_ATTACHMENTS_PER_TURN {
        return Err(ErrorResponseDto::from_code(
            VALIDATION_ERROR,
            format!("单次消息最多携带 {MAX_ATTACHMENTS_PER_TURN} 个附件"),
        ));
    }

    let mut remaining_document_context_chars = MAX_TOTAL_DOCUMENT_CONTEXT_CHARS;

    for attachment in attachments.iter_mut() {
        let owner = state
            .media_store
            .get_media_owner(attachment.id.as_str())
            .await
            .map_err(|error| ErrorResponseDto::from_code(VALIDATION_ERROR, error.to_string()))?;

        match owner.as_deref() {
            Some(owner_user_id) if owner_user_id == user_id => {
                attachment.url = state.media_store.browser_media_url(attachment.id.as_str());
                if !attachment.mime_type.starts_with("image/") {
                    let media = state
                        .media_store
                        .get_bytes(attachment.id.as_str())
                        .await
                        .map_err(|error| {
                            ErrorResponseDto::from_code(VALIDATION_ERROR, error.to_string())
                        })?
                        .ok_or_else(|| {
                            ErrorResponseDto::from_code(
                                VALIDATION_ERROR,
                                "Uploaded document could not be found",
                            )
                        })?;

                    attachment.mime_type = media.content_type.clone();
                    attachment.size_bytes = media.size_bytes;
                    attachment.extracted_text = crate::document_text::extract_supported_document_text(
                        media.bytes.as_slice(),
                        attachment.mime_type.as_str(),
                        attachment.name.as_str(),
                    )
                    .map_err(|error| ErrorResponseDto::from_code(VALIDATION_ERROR, error))?;
                    if let Some(text) = attachment.extracted_text.as_deref() {
                        if remaining_document_context_chars == 0 {
                            attachment.extracted_text =
                                Some("[本轮文档内容超出总上下文限制，已省略]".to_string());
                        } else {
                            let capped =
                                truncate_document_text(text, remaining_document_context_chars);
                            remaining_document_context_chars =
                                remaining_document_context_chars.saturating_sub(
                                    capped.chars().count().min(remaining_document_context_chars),
                                );
                            attachment.extracted_text = Some(capped);
                        }
                    }
                }
            }
            _ => {
                return Err(ErrorResponseDto::from_code(
                    ATTACHMENT_ACCESS_DENIED,
                    "One or more uploaded attachments do not belong to the current user",
                ));
            }
        }
    }

    Ok(())
}

async fn bind_attachments_to_session(
    state: &AppState,
    user_id: &str,
    session_id: &str,
    payload: &ChatRequestDto,
) -> Result<(), ErrorResponseDto> {
    let object_keys = payload
        .attachments
        .as_ref()
        .map(|attachments| {
            attachments
                .iter()
                .map(|attachment| attachment.id.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    state
        .media_store
        .assign_session_to_existing_objects(user_id, session_id, object_keys.as_slice())
        .await
        .map_err(|error| ErrorResponseDto::from_code(VALIDATION_ERROR, error.to_string()))
}

async fn ensure_session_exists(
    state: &AppState,
    user_id: &str,
    session_id: &str,
) -> Result<(), ErrorResponseDto> {
    state
        .chat_store
        .ensure_session(user_id, session_id)
        .await
        .map_err(|error| ErrorResponseDto::from_code(VALIDATION_ERROR, error.to_string()))
}

pub async fn send_message(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Json(mut payload): Json<ChatRequestDto>,
) -> impl IntoResponse {
    if let Err(error) = normalize_attachments(&state, auth.user_id(), &mut payload).await {
        return (StatusCode::FORBIDDEN, Json(error)).into_response();
    }
    if let Err(error) =
        ensure_session_exists(&state, auth.user_id(), payload.session_id.as_str()).await
    {
        return (StatusCode::BAD_REQUEST, Json(error)).into_response();
    }
    if let Err(error) = bind_attachments_to_session(
        &state,
        auth.user_id(),
        payload.session_id.as_str(),
        &payload,
    )
    .await
    {
        return (StatusCode::BAD_REQUEST, Json(error)).into_response();
    }

    if let Some(selected_text_model) = payload.text_model.as_ref() {
        if let Err(error) =
            validate_text_model_selection(&state, auth.user_id(), selected_text_model).await
        {
            return (StatusCode::BAD_REQUEST, Json(error)).into_response();
        }
    }

    if let Some(selected_tools) = payload.tool_list.as_ref() {
        if let Err(error) = validate_selected_tool_list(selected_tools) {
            return (StatusCode::BAD_REQUEST, Json(error)).into_response();
        }

        for selected_tool in selected_tools {
            if let Err(error) = validate_tool_selection(&state, auth.user_id(), selected_tool).await
            {
                return (StatusCode::BAD_REQUEST, Json(error)).into_response();
            }
        }
    }

    match state
        .chat_service
        .start_turn(payload.into_chat_request(auth.user_id().to_string()))
        .await
    {
        Ok(accepted) => (
            StatusCode::OK,
            Json(ChatAcceptedResponseDto::from(accepted)),
        )
            .into_response(),
        Err(error) => chat_service_error_response_from_error(error),
    }
}

pub async fn stream(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(session_id): Path<String>,
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
        let descriptor = if error.message == "Session not found" {
            SESSION_NOT_FOUND
        } else {
            AUTHORIZATION_DENIED
        };
        let status = if error.message == "Session not found" {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::FORBIDDEN
        };
        return error_response(status, descriptor, error.message);
    }

    let mut receiver = match state
        .chat_service
        .subscribe(auth.user_id(), session_id.as_str())
        .await
    {
        Ok(receiver) => receiver,
        Err(error) => return chat_service_error_response_from_error(error),
    };

    let event_stream = async_stream::stream! {
        loop {
            match receiver.recv().await {
                Ok(payload) => {
                    let envelope = ChatEventEnvelope::new(payload);
                    yield Ok::<Event, std::convert::Infallible>(
                        Event::default().event("stream_event").data(envelope.data),
                    );
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Sse::new(event_stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}
