use time::UtcOffset;
use tracing_subscriber::fmt::time::FormatTime;

struct ShanghaiTimer;

impl FormatTime for ShanghaiTimer {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        let now = time::OffsetDateTime::now_utc()
            .to_offset(UtcOffset::from_hms(8, 0, 0).expect("valid Shanghai UTC offset"));
        write!(
            w,
            "{}",
            now.format(&time::format_description::well_known::Rfc3339)
                .expect("format Shanghai timestamp")
        )
    }
}

pub fn init() {
    ::tracing_subscriber::fmt()
        .with_timer(ShanghaiTimer)
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| {
                "openchat_api=info,openchat_core=info,tower_http=info".to_string()
            }),
        )
        .init();
}
