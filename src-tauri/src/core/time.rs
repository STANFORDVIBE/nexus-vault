use std::time::{SystemTime, UNIX_EPOCH};

/// Unix time in milliseconds (the format used for every timestamp in the vault).
pub fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}
