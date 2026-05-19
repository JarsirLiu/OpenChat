pub fn init() {
    ::tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "openchat_server=info,tower_http=info".to_string()),
        )
        .init();
}
