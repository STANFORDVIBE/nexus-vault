use std::time::{SystemTime, UNIX_EPOCH};

/// Unix time in milliseconds (the format used for every timestamp in the vault).
pub fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// UTC calendar date and time of a unix-millis timestamp (civil-from-days, Howard Hinnant).
pub fn civil_utc(millis: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = millis.div_euclid(86_400_000);
    let secs = millis.rem_euclid(86_400_000) / 1000;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (
        year,
        month,
        day,
        (secs / 3600) as u32,
        (secs / 60 % 60) as u32,
        (secs % 60) as u32,
    )
}

/// `2026-11-01` for file names.
pub fn date_stamp(millis: i64) -> String {
    let (y, m, d, ..) = civil_utc(millis);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_known_timestamps() {
        assert_eq!(civil_utc(0), (1970, 1, 1, 0, 0, 0));
        // 2026-11-01 18:42:23 UTC
        assert_eq!(civil_utc(1_793_558_543_000), (2026, 11, 1, 18, 42, 23));
        assert_eq!(date_stamp(951_782_400_000), "2000-02-29");
    }
}
