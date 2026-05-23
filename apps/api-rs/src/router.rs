use axum::{
    http::{header, HeaderName, HeaderValue, Method},
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{config::AppConfig, handlers, state::AppState};

pub fn build_router(state: AppState, config: &AppConfig) -> Router {
    let router = Router::new()
        .route("/health", get(handlers::admin::health))
        .route("/api/auth/register", post(handlers::auth::register))
        .route("/api/auth/login", post(handlers::auth::login))
        .route("/api/auth/me", get(handlers::auth::me))
        .route("/api/auth/csrf", get(handlers::auth::csrf_token))
        .route("/api/auth/refresh", post(handlers::auth::refresh))
        .route("/api/auth/logout", post(handlers::auth::logout))
        .route("/api/list_models", get(handlers::catalog::list_models))
        .route("/api/list_tools", get(handlers::catalog::list_tools))
        .route(
            "/api/user-provider-api-keys",
            get(handlers::auth::list_user_provider_api_keys)
                .put(handlers::auth::upsert_user_provider_api_key),
        )
        .route(
            "/api/user-provider-api-keys/:provider_key",
            get(handlers::auth::get_user_provider_api_key),
        )
        .route(
            "/api/custom-models",
            get(handlers::auth::list_user_custom_models)
                .post(handlers::auth::create_user_custom_model),
        )
        .route(
            "/api/custom-models/:model_config_id",
            axum::routing::delete(handlers::auth::delete_user_custom_model),
        )
        .route("/api/media/*path", get(handlers::media::get_media))
        .route("/api/sessions", get(handlers::sessions::list_sessions))
        .route(
            "/api/sessions/:session_id",
            get(handlers::sessions::get_session)
                .put(handlers::sessions::rename_session)
                .delete(handlers::sessions::delete_session),
        )
        .route("/api/uploads/images", post(handlers::upload::upload_images))
        .route("/api/uploads/files", post(handlers::upload::upload_files))
        .route("/api/chat", post(handlers::chat::send_message))
        .route(
            "/api/sessions/:session_id/turns/:turn_id/interrupt",
            post(handlers::turns::interrupt_turn),
        )
        .route("/api/stream/:session_id", get(handlers::chat::stream))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::security::csrf::require_csrf,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::security::middleware::resolve_auth_context,
        ));

    let router = if config.cors_allowed_origins.is_empty() {
        router
    } else {
        let allowed_origins = config
            .cors_allowed_origins
            .iter()
            .filter_map(|origin| HeaderValue::from_str(origin).ok())
            .collect::<Vec<_>>();

        router.layer(
            CorsLayer::new()
                .allow_credentials(true)
                .allow_origin(allowed_origins)
                .allow_headers([
                    header::AUTHORIZATION,
                    header::CONTENT_TYPE,
                    HeaderName::from_static("x-csrf-token"),
                ])
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::DELETE,
                    Method::OPTIONS,
                ]),
        )
    };

    router.with_state(state)
}
