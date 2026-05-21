use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};

const SHANGHAI_UTC_OFFSET_HOURS: i8 = 8;

fn shanghai_offset() -> UtcOffset {
    UtcOffset::from_hms(SHANGHAI_UTC_OFFSET_HOURS, 0, 0).expect("valid Shanghai UTC offset")
}

pub fn format_millis_timestamp(raw: &str) -> String {
    raw.parse::<i64>()
        .ok()
        .and_then(|millis| {
            OffsetDateTime::from_unix_timestamp_nanos((millis as i128) * 1_000_000).ok()
        })
        .map(|datetime| {
            datetime
                .to_offset(shanghai_offset())
                .format(&Rfc3339)
                .expect("format Shanghai timestamp")
        })
        .unwrap_or_else(|| raw.to_string())
}
