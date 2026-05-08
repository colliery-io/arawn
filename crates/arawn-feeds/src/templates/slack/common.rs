//! Shared two-pass + dual-layer archive logic used by every Slack
//! "archive a conversation" template (`channel-archive`, `dm-archive`,
//! and future `mpim-archive`).
//!
//! The conversation has already been resolved to a Slack channel id
//! (`C`/`G` for channels, `D` for DMs, `M` for mpims). This helper
//! does the rest: pass-1 history fetch + day-file writes, pass-2
//! per-thread reply fetch + thread-file writes, cursor management.

use std::path::Path;
use std::time::Instant;

use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Map, Value, json};

use crate::clients::SlackFeedClient;
use crate::error::FeedError;
use crate::template::RunOutcome;
use crate::types::RunSummary;

/// Two-pass dual-layer archive of a single Slack conversation.
///
/// Cursor shape:
/// ```json
/// {
///   "latest_ts": "<ts of newest top-level seen, or null>",
///   "threads": { "<parent_ts>": "<last_reply_ts | null>", ... }
/// }
/// ```
///
/// Disk layout (relative to `feed_dir`):
/// - `<YYYY-MM-DD>.jsonl` — top-level messages by their own Slack ts.
/// - `threads/<parent_ts>.jsonl` — parent + every reply chronologically.
pub async fn archive_channel_with_threads(
    slack: &dyn SlackFeedClient,
    channel_id: &str,
    feed_dir: &Path,
    cursor: &Value,
) -> Result<RunOutcome, FeedError> {
    let started = Instant::now();

    let mut new_latest_ts: Option<String> = cursor
        .get("latest_ts")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let mut threads_cursor: Map<String, Value> = cursor
        .get("threads")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let mut total_items: u64 = 0;
    let mut total_bytes: u64 = 0;

    // ── Pass 1: top-level messages via conversations.history ─────
    let oldest_top = new_latest_ts.clone();
    let history = slack
        .channel_history(channel_id, oldest_top.as_deref())
        .await?;

    if !history.messages.is_empty() {
        for msg in &history.messages {
            let ts = msg.get("ts").and_then(|v| v.as_str()).ok_or_else(|| {
                FeedError::Schema("slack message missing `ts` string field".into())
            })?;
            // Top-level → day file, partitioned by the message's
            // OWN ts (not fetch time).
            let bytes = append_message_to_day(feed_dir, msg, ts)?;
            total_items += 1;
            total_bytes += bytes;

            // If the message has replies, register the thread for
            // pass 2. Also seed the thread file with the parent so
            // the file is self-contained even if we never see new
            // replies later.
            if has_replies(msg) {
                let parent_ts = ts.to_string();
                if !threads_cursor.contains_key(&parent_ts) {
                    threads_cursor.insert(parent_ts.clone(), Value::Null);
                    let parent_bytes = append_message_to_thread(feed_dir, &parent_ts, msg)?;
                    total_bytes += parent_bytes;
                }
            }
        }
        new_latest_ts = history.next_cursor_ts.or(new_latest_ts);
    }

    // ── Pass 2: per-thread reply fetch ───────────────────────────
    // Each thread advances independently. Failure on one thread is
    // logged but does NOT abort the run or block the channel cursor
    // or other threads.
    let parent_tss: Vec<String> = threads_cursor.keys().cloned().collect();
    for parent_ts in parent_tss {
        let prior = threads_cursor
            .get(&parent_ts)
            .and_then(|v| v.as_str())
            .map(str::to_string);
        match slack
            .thread_replies(channel_id, &parent_ts, prior.as_deref())
            .await
        {
            Ok(page) => {
                for msg in &page.messages {
                    let ts = msg
                        .get("ts")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    // Skip the parent if we just seeded it from
                    // history (first call returns parent + replies).
                    if ts == parent_ts && prior.is_none() {
                        continue;
                    }
                    let bytes = append_message_to_thread(feed_dir, &parent_ts, msg)?;
                    total_items += 1;
                    total_bytes += bytes;
                }
                if let Some(new_cursor) = page.next_cursor_ts {
                    threads_cursor.insert(parent_ts.clone(), Value::String(new_cursor));
                }
            }
            Err(e) => {
                tracing::warn!(
                    parent_ts = %parent_ts,
                    error = %e,
                    "thread fetch failed; cursor unchanged for this thread"
                );
            }
        }
    }

    let new_cursor = json!({
        "latest_ts": new_latest_ts.map(Value::String).unwrap_or(Value::Null),
        "threads": Value::Object(threads_cursor),
    });

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

// ── Disk helpers ────────────────────────────────────────────────────

fn append_message_to_day(feed_dir: &Path, msg: &Value, ts: &str) -> Result<u64, FeedError> {
    let day = ts_to_yyyy_mm_dd(ts)?;
    let path = feed_dir.join(format!("{day}.jsonl"));
    append_line(&path, msg)
}

fn append_message_to_thread(
    feed_dir: &Path,
    parent_ts: &str,
    msg: &Value,
) -> Result<u64, FeedError> {
    let dir = feed_dir.join("threads");
    std::fs::create_dir_all(&dir)
        .map_err(|e| FeedError::Storage(format!("create threads dir: {e}")))?;
    let path = dir.join(format!("{parent_ts}.jsonl"));
    append_line(&path, msg)
}

fn append_line(path: &Path, msg: &Value) -> Result<u64, FeedError> {
    use std::io::Write;
    let line = serde_json::to_string(msg)
        .map_err(|e| FeedError::Storage(format!("serialize message: {e}")))?;
    let bytes = format!("{line}\n");
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| FeedError::Storage(format!("open {}: {e}", path.display())))?;
    f.write_all(bytes.as_bytes())
        .map_err(|e| FeedError::Storage(format!("append: {e}")))?;
    Ok(bytes.len() as u64)
}

fn has_replies(msg: &Value) -> bool {
    msg.get("reply_count")
        .and_then(|v| v.as_u64())
        .map(|n| n > 0)
        .unwrap_or(false)
}

/// Parse Slack's float-string `ts` (`"1715000000.001234"`) and format
/// it as `YYYY-MM-DD` UTC.
fn ts_to_yyyy_mm_dd(ts: &str) -> Result<String, FeedError> {
    let secs_str = ts.split('.').next().unwrap_or(ts);
    let secs: i64 = secs_str
        .parse()
        .map_err(|_| FeedError::Schema(format!("bad slack ts '{ts}'")))?;
    let dt: DateTime<Utc> = Utc
        .timestamp_opt(secs, 0)
        .single()
        .ok_or_else(|| FeedError::Schema(format!("ts {ts} out of range")))?;
    Ok(dt.format("%Y-%m-%d").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ts_to_yyyy_mm_dd_parses_slack_format() {
        let dt = Utc.with_ymd_and_hms(2026, 5, 8, 12, 30, 0).unwrap();
        let ts = format!("{}.000123", dt.timestamp());
        assert_eq!(ts_to_yyyy_mm_dd(&ts).unwrap(), "2026-05-08");
    }

    #[test]
    fn ts_to_yyyy_mm_dd_rejects_garbage() {
        assert!(matches!(
            ts_to_yyyy_mm_dd("not-a-ts"),
            Err(FeedError::Schema(_))
        ));
    }

    #[test]
    fn has_replies_detects_reply_count() {
        assert!(has_replies(&json!({ "reply_count": 3 })));
        assert!(!has_replies(&json!({ "reply_count": 0 })));
        assert!(!has_replies(&json!({})));
    }
}
