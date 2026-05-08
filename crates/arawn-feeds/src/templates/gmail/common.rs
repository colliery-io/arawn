//! Shared Gmail archive helper.
//!
//! All three Gmail templates (`inbox-archive`, `sender-filter`,
//! `label-archive`) collapse to: build a Gmail search query, list ids,
//! fetch each new id full, write `<YYYY-MM-DD>/<message_id>.json`.
//! This helper is the only place that logic lives.
//!
//! Layout produced:
//!
//! ```text
//! <feed_dir>/
//!   ├── meta.json                  # cursor: { latest_internal_date }
//!   ├── 2026-05-08/
//!   │   ├── <msg_id_a>.json        # raw Gmail Message JSON, format=full
//!   │   └── <msg_id_b>.json
//!   └── 2026-05-07/
//!       └── <msg_id_c>.json
//! ```
//!
//! Files are partitioned by Gmail's `internalDate` (the canonical send
//! time), not by fetch time, so the archive is stable across re-runs.
//!
//! Idempotence: if `<YYYY-MM-DD>/<id>.json` already exists, the helper
//! skips the `messages.get` API call entirely. That makes re-runs
//! cheap and protects the archive from thrash if the cursor gets
//! reset.
//!
//! Cursor: `latest_internal_date` is the highest `internalDate` (ms
//! since epoch, i64) the helper has ever persisted. The next run uses
//! it to short-circuit the per-message check and to advance the
//! cursor monotonically. `internalDate` is what Gmail itself uses for
//! list ordering, so it's the right key.

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Value, json};

use crate::clients::GmailFeedClient;
use crate::error::FeedError;
use crate::template::RunOutcome;
use crate::types::RunSummary;

/// Hard cap on how many ids we'll pull per run. Gmail caps `maxResults`
/// at 500; we pick a smaller default — feeds are continual, so any
/// excess gets picked up next tick instead of fighting one giant page.
pub const DEFAULT_MAX_RESULTS: u32 = 100;

/// Run a Gmail archive over `query`, writing every new message under
/// `feed_dir`. Shared by every Gmail template.
///
/// `query` should already include any time bound the caller cares
/// about (e.g. `"newer_than:7d"`); we don't add one here so each
/// template owns the exact semantics of its window.
pub async fn archive_query(
    gmail: Arc<dyn GmailFeedClient>,
    feed_dir: &Path,
    query: &str,
    cursor: &Value,
) -> Result<RunOutcome, FeedError> {
    let started = Instant::now();

    let prior_latest: Option<i64> = cursor
        .get("latest_internal_date")
        .and_then(|v| v.as_i64());

    let ids = gmail
        .list_message_ids(query, DEFAULT_MAX_RESULTS)
        .await?;

    let mut total_items: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut new_latest: Option<i64> = prior_latest;

    for id in &ids {
        // We don't yet know the internalDate without fetching, but we
        // can skip the fetch entirely if we've already archived this
        // id under any day partition. Cheap probe via filesystem.
        if existing_message_path(feed_dir, id).is_some() {
            continue;
        }

        let msg = match gmail.get_message(id).await {
            Ok(m) => m,
            // One bad message shouldn't poison the whole run. The
            // template can still advance for everything else.
            Err(FeedError::Provider(e)) | Err(FeedError::Schema(e)) => {
                tracing::warn!(target: "arawn::feeds", %id, error=%e, "skipping gmail message");
                continue;
            }
            Err(other) => return Err(other),
        };

        let internal_date = parse_internal_date(&msg).ok_or_else(|| {
            FeedError::Schema(format!("message {id} missing internalDate"))
        })?;
        if let Some(prev) = prior_latest
            && internal_date <= prev
        {
            // We've already passed the cursor; everything older has
            // either been written or intentionally skipped. Stop early
            // — Gmail returns ids most-recent-first, so we won't see
            // newer ones beyond this point in the same page.
            break;
        }

        let day = ms_to_yyyy_mm_dd(internal_date)?;
        let day_dir = feed_dir.join(&day);
        std::fs::create_dir_all(&day_dir).map_err(|e| {
            FeedError::Storage(format!("create {}: {e}", day_dir.display()))
        })?;
        let path = day_dir.join(format!("{id}.json"));
        let bytes = write_message_file(&path, &msg)?;
        total_items += 1;
        total_bytes += bytes;

        if new_latest.map(|n| internal_date > n).unwrap_or(true) {
            new_latest = Some(internal_date);
        }
    }

    let new_cursor = json!({ "latest_internal_date": new_latest });
    let status = if total_items == 0 {
        "no-new-items".to_string()
    } else {
        "ok".to_string()
    };

    Ok(RunOutcome {
        cursor: new_cursor,
        summary: RunSummary {
            items_written: total_items,
            bytes_written: total_bytes,
            duration: started.elapsed(),
        },
        status,
    })
}

/// Probe every day partition under `feed_dir` for an existing
/// `<id>.json`. We don't know the day up front (that's what the fetch
/// tells us), so we scan. Cheap on N=tens of partition dirs; for very
/// long-lived feeds we could keep an in-memory id index, but that's
/// premature.
fn existing_message_path(feed_dir: &Path, id: &str) -> Option<std::path::PathBuf> {
    let needle = format!("{id}.json");
    let entries = std::fs::read_dir(feed_dir).ok()?;
    for entry in entries.flatten() {
        if !entry.file_type().ok()?.is_dir() {
            continue;
        }
        let candidate = entry.path().join(&needle);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn parse_internal_date(msg: &Value) -> Option<i64> {
    // google-gmail1 serializes `internalDate` as a string of millis.
    // Be lenient and accept a number too.
    let v = msg.get("internalDate")?;
    if let Some(s) = v.as_str() {
        return s.parse::<i64>().ok();
    }
    v.as_i64()
}

fn ms_to_yyyy_mm_dd(ms: i64) -> Result<String, FeedError> {
    let secs = ms / 1000;
    let nanos = ((ms % 1000) * 1_000_000) as u32;
    let dt: DateTime<Utc> = Utc
        .timestamp_opt(secs, nanos)
        .single()
        .ok_or_else(|| FeedError::Schema(format!("internalDate {ms} out of range")))?;
    Ok(dt.format("%Y-%m-%d").to_string())
}

fn write_message_file(path: &Path, msg: &Value) -> Result<u64, FeedError> {
    let body = serde_json::to_vec_pretty(msg)
        .map_err(|e| FeedError::Storage(format!("serialize message: {e}")))?;
    let len = body.len() as u64;
    // Atomic write — survives a crash mid-write without leaving a
    // partial JSON file the next run would mistake for "already
    // archived."
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &body)
        .map_err(|e| FeedError::Storage(format!("write {}: {e}", tmp.display())))?;
    std::fs::rename(&tmp, path)
        .map_err(|e| FeedError::Storage(format!("rename {}: {e}", path.display())))?;
    Ok(len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ms_to_yyyy_mm_dd_basic() {
        let ms = chrono::Utc
            .with_ymd_and_hms(2026, 5, 8, 12, 0, 0)
            .unwrap()
            .timestamp_millis();
        assert_eq!(ms_to_yyyy_mm_dd(ms).unwrap(), "2026-05-08");
    }

    #[test]
    fn parse_internal_date_string_or_number() {
        let s = json!({ "internalDate": "1778414400000" });
        assert_eq!(parse_internal_date(&s), Some(1778414400000));
        let n = json!({ "internalDate": 1778414400000_i64 });
        assert_eq!(parse_internal_date(&n), Some(1778414400000));
        let none = json!({ "id": "abc" });
        assert_eq!(parse_internal_date(&none), None);
    }
}
