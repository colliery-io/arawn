//! Helpers for converting a local-date boundary into a UTC instant.
//!
//! Used by `Ceremony::period_window` implementations. DST gotcha:
//! when a clock moves forward at 00:00 (rare but real), the local
//! midnight may not exist; when it falls back, midnight is
//! ambiguous. We resolve via `single()` and fall back to the
//! earliest valid offset, mirroring `chrono`'s recommended pattern.
//! The result is that tablets straddling a DST flip can have a
//! slightly long or short window — accepted as truthful per
//! ARAWN-I-0052.

use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;

use crate::CeremonyError;

/// Resolve `date 00:00` in `tz` to a UTC instant.
///
/// Picks the unique offset when possible; on ambiguous wall-clock
/// times prefers the earliest offset; on non-existent wall-clock
/// times (spring-forward gap) returns the next valid local
/// midnight projected forward into the gap.
pub fn local_midnight_utc(date: NaiveDate, tz: Tz) -> Result<DateTime<Utc>, CeremonyError> {
    let midnight = date.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    match tz.from_local_datetime(&midnight) {
        chrono::LocalResult::Single(dt) => Ok(dt.with_timezone(&Utc)),
        chrono::LocalResult::Ambiguous(earliest, _) => Ok(earliest.with_timezone(&Utc)),
        chrono::LocalResult::None => {
            // Spring-forward at midnight: advance by 1h until valid.
            // In practice DST transitions don't happen at 00:00 in
            // most zones, so this branch is mostly defensive.
            for hours in 1..=4 {
                let bumped = midnight + chrono::Duration::hours(hours);
                if let chrono::LocalResult::Single(dt) = tz.from_local_datetime(&bumped) {
                    return Ok(dt.with_timezone(&Utc));
                }
            }
            Err(CeremonyError::Other(format!(
                "no valid local midnight near {date} in tz {tz}",
            )))
        }
    }
}

/// Convenience for daily plugin: window covering one local day.
pub fn day_window_utc(
    date: NaiveDate,
    tz: Tz,
) -> Result<(DateTime<Utc>, DateTime<Utc>), CeremonyError> {
    let start = local_midnight_utc(date, tz)?;
    let next = date
        .succ_opt()
        .ok_or_else(|| CeremonyError::Other(format!("date {date} has no successor")))?;
    let end = local_midnight_utc(next, tz)?;
    Ok((start, end))
}

/// Convenience for weekly/retro plugins: window covering one local
/// ISO week starting Monday.
pub fn iso_week_window_utc(
    monday: NaiveDate,
    tz: Tz,
) -> Result<(DateTime<Utc>, DateTime<Utc>), CeremonyError> {
    let start = local_midnight_utc(monday, tz)?;
    let next_monday = monday + chrono::Duration::days(7);
    let end = local_midnight_utc(next_monday, tz)?;
    Ok((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::{America::Los_Angeles, UTC as TzUtc};

    #[test]
    fn day_window_utc_zone() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 19).unwrap();
        let (start, end) = day_window_utc(date, TzUtc).unwrap();
        assert_eq!(start.to_rfc3339(), "2026-05-19T00:00:00+00:00");
        assert_eq!(end.to_rfc3339(), "2026-05-20T00:00:00+00:00");
    }

    #[test]
    fn day_window_pacific() {
        // 2026-05-19 in PDT (UTC-7) → window starts at 07:00 UTC.
        let date = NaiveDate::from_ymd_opt(2026, 5, 19).unwrap();
        let (start, end) = day_window_utc(date, Los_Angeles).unwrap();
        assert_eq!(start.to_rfc3339(), "2026-05-19T07:00:00+00:00");
        assert_eq!(end.to_rfc3339(), "2026-05-20T07:00:00+00:00");
    }

    #[test]
    fn iso_week_window_pacific() {
        // ISO week 2026-W21 starts Monday 2026-05-18.
        let monday = NaiveDate::from_isoywd_opt(2026, 21, chrono::Weekday::Mon).unwrap();
        let (start, end) = iso_week_window_utc(monday, Los_Angeles).unwrap();
        assert_eq!(start.to_rfc3339(), "2026-05-18T07:00:00+00:00");
        assert_eq!(end.to_rfc3339(), "2026-05-25T07:00:00+00:00");
    }

    #[test]
    fn dst_spring_forward_ambiguity_resolved() {
        // US spring-forward: 2026-03-08 02:00 → 03:00. Midnight is
        // single — make sure we still produce a valid instant for
        // the day window crossing that boundary.
        let date = NaiveDate::from_ymd_opt(2026, 3, 8).unwrap();
        let (start, end) = day_window_utc(date, Los_Angeles).unwrap();
        // 2026-03-08 is the DST flip; 00:00 PST → 08:00 UTC;
        // next day 00:00 PDT → 07:00 UTC. So the window is 23h.
        let diff = end - start;
        assert_eq!(diff.num_hours(), 23);
    }
}
