//! `Retry-After` header parsing.
//!
//! RFC 7231 §7.1.3 allows two forms:
//! - `delta-seconds`: a non-negative integer (e.g. `120`).
//! - `HTTP-date`: an RFC 1123 date (e.g. `Fri, 31 Dec 1999 23:59:59 GMT`).
//!
//! Lives in arawn-integrations so adapters (Atlassian, future raw-HTTP
//! providers) can use it without depending on arawn-feeds. arawn-feeds
//! re-exports it.

use std::time::Duration;

use chrono::{DateTime, Utc};

/// Parse a `Retry-After` header value. HTTP-date forms are computed
/// against the current wall clock and clamped to zero if in the past.
/// Returns `None` for missing/empty/unparseable values.
pub fn parse_retry_after(raw: Option<&str>) -> Option<Duration> {
    parse_retry_after_at(raw, Utc::now())
}

pub(crate) fn parse_retry_after_at(raw: Option<&str>, now: DateTime<Utc>) -> Option<Duration> {
    let s = raw?.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(secs) = s.parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }
    let when = DateTime::parse_from_rfc2822(s).ok()?.with_timezone(&Utc);
    let delta = when - now;
    if delta <= chrono::Duration::zero() {
        Some(Duration::ZERO)
    } else {
        Some(Duration::from_secs(delta.num_seconds() as u64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> DateTime<Utc> {
        s.parse::<DateTime<Utc>>().unwrap()
    }

    #[test]
    fn delta_seconds() {
        assert_eq!(parse_retry_after(Some("120")), Some(Duration::from_secs(120)));
        assert_eq!(parse_retry_after(Some("  30 ")), Some(Duration::from_secs(30)));
        assert_eq!(parse_retry_after(Some("0")), Some(Duration::ZERO));
    }

    #[test]
    fn http_date_future() {
        let now = at("2026-05-11T12:00:00Z");
        let raw = Some("Mon, 11 May 2026 12:01:00 GMT");
        assert_eq!(parse_retry_after_at(raw, now), Some(Duration::from_secs(60)));
    }

    #[test]
    fn http_date_past_clamps_to_zero() {
        let now = at("2026-05-11T12:00:00Z");
        let raw = Some("Mon, 11 May 2026 11:00:00 GMT");
        assert_eq!(parse_retry_after_at(raw, now), Some(Duration::ZERO));
    }

    #[test]
    fn missing_or_garbage() {
        assert_eq!(parse_retry_after(None), None);
        assert_eq!(parse_retry_after(Some("")), None);
        assert_eq!(parse_retry_after(Some("not a date")), None);
    }
}
