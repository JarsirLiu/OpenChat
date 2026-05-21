use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};

const SHANGHAI_UTC_OFFSET_HOURS: i8 = 8;

fn shanghai_offset() -> UtcOffset {
    UtcOffset::from_hms(SHANGHAI_UTC_OFFSET_HOURS, 0, 0).expect("valid Shanghai UTC offset")
}

pub fn now_string() -> String {
    OffsetDateTime::now_utc()
        .to_offset(shanghai_offset())
        .format(&Rfc3339)
        .expect("format Shanghai timestamp")
}
