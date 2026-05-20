use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

use crate::{
    http::errors::ErrorResponseDto,
    security::extractors::MaybeCurrentUser,
    state::AppState,
};

#[derive(Deserialize)]
pub struct MediaAccessQuery {
    sig: Option<String>,
}

pub async fn get_media(
    State(state): State<AppState>,
    MaybeCurrentUser(auth): MaybeCurrentUser,
    Path(path): Path<String>,
    Query(query): Query<MediaAccessQuery>,
) -> impl IntoResponse {
    let owner_user_id = match state.media_store.get_media_owner(path.as_str()).await {
        Ok(Some(user_id)) => user_id,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponseDto {
                    message: "Media not found".to_string(),
                }),
            )
                .into_response()
        }
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponseDto {
                    message: error.to_string(),
                }),
            )
                .into_response()
        }
    };

    let browser_authorized = auth
        .as_ref()
        .map(|auth| auth.user_id() == owner_user_id.as_str())
        .unwrap_or(false);
    let signed_authorized = query
        .sig
        .as_deref()
        .map(|signature| state.media_store.verify_media_signature(path.as_str(), signature))
        .unwrap_or(false);

    if !browser_authorized && !signed_authorized {
        let status = if auth.is_some() {
            StatusCode::FORBIDDEN
        } else {
            StatusCode::UNAUTHORIZED
        };
        return (
            status,
            Json(ErrorResponseDto {
                message: "Media access denied".to_string(),
            }),
        )
            .into_response();
    }

    let Some(media) = (match state.media_store.get_bytes(path.as_str()).await {
        Ok(media) => media,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponseDto {
                    message: error.to_string(),
                }),
            )
                .into_response()
        }
    }) else {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponseDto {
                message: "Media not found".to_string(),
            }),
        )
            .into_response();
    };

    let mut headers = HeaderMap::new();
    if let Ok(value) = header::HeaderValue::from_str(media.content_type.as_str()) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    if let Ok(value) = header::HeaderValue::from_str(media.size_bytes.to_string().as_str()) {
        headers.insert(header::CONTENT_LENGTH, value);
    }
    headers.insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, max-age=3600"),
    );

    (StatusCode::OK, headers, Body::from(media.bytes)).into_response()
}
