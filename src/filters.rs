use jiff::{tz::TimeZone, Timestamp};

pub fn datetime(s: &Timestamp, _: &dyn askama::Values) -> askama::Result<String> {
    let tz = TimeZone::system();
    Ok(s.to_zoned(tz).strftime("%d %b %Y %R").to_string())
}
