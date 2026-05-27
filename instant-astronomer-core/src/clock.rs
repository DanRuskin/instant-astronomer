//! UTC + local-clock helpers used by the configuration tray.
//!
//! Extracted from `lib.rs` to keep that file under the workspace
//! line-count guardrail.

/// Current UTC unix time in milliseconds. Wrapped here so the entry points
/// don't repeat the `web_time` plumbing.
pub fn current_unix_ms() -> i64 {
    web_time::SystemTime::now()
        .duration_since(web_time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Build the "UTC HH:MM · local HH:MM" status string shown in the
/// control panel so the user can verify the app has them located in
/// the right place at the right time.
///
/// The "local" half is **solar time** — UTC offset by `longitude /
/// 15 hours`. That's not the user's legal-civil time (which would
/// require a tz database to look up the offset from coords + the DST
/// rules) but it's close enough that someone in California will see
/// roughly Pacific time, someone in London will see roughly UK time,
/// etc. Worth ~30 minutes of error vs. the alternative of bundling
/// `tzf-rs` (~5 MB of polygon data) into the WASM blob.
pub(crate) fn format_clock_label(timestamp_ms: i64, offset_minutes: i32) -> String {
    let utc_h = ((timestamp_ms / 3_600_000) % 24 + 24) % 24;
    let utc_m = ((timestamp_ms / 60_000) % 60 + 60) % 60;
    // Local wall clock = UTC + platform-reported offset. The platform
    // applies DST, so we just add minutes blindly here.
    let local_ms = timestamp_ms + (offset_minutes as i64) * 60_000;
    let l_h = ((local_ms / 3_600_000) % 24 + 24) % 24;
    let l_m = ((local_ms / 60_000) % 60 + 60) % 60;
    format!(
        "UTC {:02}:{:02} · local {:02}:{:02}",
        utc_h, utc_m, l_h, l_m
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Local clock follows the platform-reported offset, DST included.
    /// 1700000000000 ms is 2023-11-14T22:13:20Z; with offset -480 (PST)
    /// that's 14:13 local. With +330 (IST) that's 03:43 next-day local
    /// — wrap correctly.
    #[test]
    fn format_clock_label_applies_offset_with_wrap() {
        let s = format_clock_label(1_700_000_000_000, -480);
        assert!(s.contains("UTC 22:13"), "got: {s}");
        assert!(s.contains("local 14:13"), "got: {s}");

        let s = format_clock_label(1_700_000_000_000, 330);
        assert!(s.contains("local 03:43"), "got: {s}");
    }
}
