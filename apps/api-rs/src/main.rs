mod config;
mod handlers;
mod http;
mod router;
mod security;
mod state;
mod system_provider_registry;
mod tracing_setup;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    tracing_setup::init();

    let config = config::AppConfig::from_env();
    let state = state::AppState::new(&config).await?;
    let router = router::build_router(state, &config);

    ::tracing::info!("openchat-server listening on http://{}", config.bind_addr);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}
