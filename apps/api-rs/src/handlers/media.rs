use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::Deserialize;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use crate::{
    http::errors::{
        error_response, AUTHENTICATION_REQUIRED, AUTHORIZATION_DENIED, INTERNAL_ERROR,
        MEDIA_NOT_FOUND,
    },
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
    request_headers: HeaderMap,
    Path(path): Path<String>,
    Query(query): Query<MediaAccessQuery>,
) -> impl IntoResponse {
    let owner_user_id = match state.media_store.get_media_owner(path.as_str()).await {
        Ok(Some(user_id)) => user_id,
        Ok(None) => {
            return error_response(StatusCode::NOT_FOUND, MEDIA_NOT_FOUND, "Media not found")
        }
        Err(error) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                INTERNAL_ERROR,
                error.to_string(),
            )
        }
    };

    let browser_authorized = auth
        .as_ref()
        .map(|auth| auth.user_id() == owner_user_id.as_str())
        .unwrap_or(false);
    let signed_authorized = query
        .sig
        .as_deref()
        .map(|signature| {
            state
                .media_store
                .verify_media_signature(path.as_str(), signature)
        })
        .unwrap_or(false);

    if !browser_authorized && !signed_authorized {
        let status = if auth.is_some() {
            StatusCode::FORBIDDEN
        } else {
            StatusCode::UNAUTHORIZED
        };
        let descriptor = if auth.is_some() {
            AUTHORIZATION_DENIED
        } else {
            AUTHENTICATION_REQUIRED
        };
        return error_response(status, descriptor, "Media access denied");
    }

    let Some(media) = (match state.media_store.get_bytes(path.as_str()).await {
        Ok(media) => media,
        Err(error) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                INTERNAL_ERROR,
                error.to_string(),
            )
        }
    }) else {
        return error_response(StatusCode::NOT_FOUND, MEDIA_NOT_FOUND, "Media not found");
    };

    let mut etag_hasher = DefaultHasher::new();
    path.hash(&mut etag_hasher);
    media.size_bytes.hash(&mut etag_hasher);
    media.content_type.hash(&mut etag_hasher);
    let etag = format!("\"{:x}\"", etag_hasher.finish());
    let cache_control = if browser_authorized {
        "private, max-age=86400, immutable"
    } else {
        "private, max-age=900"
    };

    let mut headers = HeaderMap::new();
    if let Ok(value) = header::HeaderValue::from_str(media.content_type.as_str()) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    if let Ok(value) = header::HeaderValue::from_str(media.size_bytes.to_string().as_str()) {
        headers.insert(header::CONTENT_LENGTH, value);
    }
    if let Ok(value) = header::HeaderValue::from_str(etag.as_str()) {
        headers.insert(header::ETAG, value);
    }
    if let Ok(value) = header::HeaderValue::from_str(cache_control) {
        headers.insert(header::CACHE_CONTROL, value);
    }

    let not_modified = request_headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.split(',').any(|candidate| candidate.trim() == etag))
        .unwrap_or(false);
    if not_modified {
        headers.remove(header::CONTENT_LENGTH);
        headers.remove(header::CONTENT_TYPE);
        return (StatusCode::NOT_MODIFIED, headers, Body::empty()).into_response();
    }

    (StatusCode::OK, headers, Body::from(media.bytes)).into_response()
}
