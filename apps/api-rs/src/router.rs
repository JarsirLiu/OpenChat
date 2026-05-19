use axum::{
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{config::MediaStorageConfig, handlers, state::AppState};

pub fn build_router(state: AppState) -> Router {
    let router = Router::new()
        .route("/health", get(handlers::admin::health))
        .route("/api/auth/register", post(handlers::auth::register))
        .route("/api/auth/login", post(handlers::auth::login))
        .route("/api/auth/me", get(handlers::auth::me))
        .route("/api/auth/refresh", post(handlers::auth::refresh))
        .route("/api/auth/logout", post(handlers::auth::logout))
        .route("/api/list_models", get(handlers::catalog::list_models))
        .route("/api/list_tools", get(handlers::catalog::list_tools))
        .route(
            "/api/provider-settings",
            get(handlers::auth::list_user_provider_settings)
                .put(handlers::auth::upsert_user_provider_setting),
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
        .route("/api/sessions", get(handlers::sessions::list_sessions))
        .route(
            "/api/sessions/:session_id",
            get(handlers::sessions::get_session)
                .put(handlers::sessions::rename_session)
                .delete(handlers::sessions::delete_session),
        )
        .route("/api/uploads/images", post(handlers::upload::upload_images))
        .route("/api/chat", post(handlers::chat::send_message))
        .route(
            "/api/sessions/:session_id/turns/:turn_id/interrupt",
            post(handlers::turns::interrupt_turn),
        )
        .route("/api/stream/:session_id", get(handlers::chat::stream))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let router = match &state.media_storage {
        MediaStorageConfig::Local { root_dir, .. } => {
            router.route_service("/media/*path", ServeDir::new(root_dir.clone()))
        }
        MediaStorageConfig::S3 { .. } => router,
    };

    router.with_state(state)
}
