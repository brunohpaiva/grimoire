pub fn datetime(s: &jiff::Timestamp, _: &dyn askama::Values) -> askama::Result<String> {
    Ok(s.strftime("%d %b %Y %R %z").to_string())
}
