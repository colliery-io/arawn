//! `slack/channel-archive` — append every message in one Slack channel
//! to JSONL, time-partitioned by day, plus per-thread reply files.
//!
//! Disk layout:
//!
//! ```text
//! slack/channel-archive/<feed_id>/
//!   ├── meta.json                       # cursor: { latest_ts, threads: {...} }
//!   ├── 2026-05-08.jsonl                # top-level messages (parents + standalone), by ts
//!   ├── 2026-05-07.jsonl
//!   └── threads/
//!       ├── 1746700000.000100.jsonl     # parent + every reply for that thread, by reply ts
//!       └── ...
//! ```
//!
//! Two-pass fetch + dual layout:
//!
//! 1. `conversations.history` returns top-level messages newer than
//!    `cursor.latest_ts`. Each message is appended to the day file
//!    matching its own Slack ts (NOT the fetch time).
//! 2. For every top-level message with `reply_count > 0`, register a
//!    thread cursor (`cursor.threads[parent_ts]`). For every entry in
//!    `cursor.threads`, call `conversations.replies` with the
//!    per-thread cursor; append each new reply to
//!    `threads/<parent_ts>.jsonl`. Threads files include the parent
//!    as line 0 the first time we touch the thread, so the file is
//!    self-contained.
//!
//! Cursor model: `{ latest_ts, threads: { <parent_ts>: <last_reply_ts | null> } }`.
//! `latest_ts` and each thread cursor advance independently — a 429
//! on one thread doesn't drop the channel cursor or block other threads.

use std::path::Path;
use std::time::Instant;

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Map, Value, json};

use crate::error::FeedError;
use crate::template::{FeedTemplate, RunOutcome, TemplateCtx};
use crate::types::{FeedDefaults, RunSummary, TemplateParams};

pub struct ChannelArchiveTemplate;

const NAME: &str = "slack/channel-archive";

#[async_trait]
impl FeedTemplate for ChannelArchiveTemplate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn validate(&self, params: &TemplateParams) -> Result<(), FeedError> {
        let channel = params.get_str("channel").ok_or_else(|| {
            FeedError::InvalidParams(
                "slack/channel-archive requires `channel` (name like '#design' or id like 'CABCDEF')"
                    .into(),
            )
        })?;
        if channel.trim().is_empty() {
            return Err(FeedError::InvalidParams(
                "slack/channel-archive `channel` cannot be empty".into(),
            ));
        }
        Ok(())
    }

    fn defaults(&self, _params: &TemplateParams) -> FeedDefaults {
        FeedDefaults {
            cadence: "*/15 * * * *".into(),
            initial_cursor: json!({ "latest_ts": Value::Null, "threads": {} }),
        }
    }

    async fn run(
        &self,
        ctx: &TemplateCtx,
        params: &TemplateParams,
        feed_dir: &Path,
        cursor: &Value,
    ) -> Result<RunOutcome, FeedError> {
        let started = Instant::now();
        let slack = ctx.clients().slack().ok_or_else(|| {
            FeedError::Auth("slack integration not connected".into())
        })?;

        let raw_channel = params
            .get_str("channel")
            .ok_or_else(|| FeedError::InvalidParams("missing `channel` param".into()))?;
        let channel_id = slack.resolve_channel(raw_channel).await?;

        // ── State carried across this run ────────────────────────────
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
            .channel_history(&channel_id, oldest_top.as_deref())
            .await?;

        if !history.messages.is_empty() {
            // Track new threads we discover so pass 2 picks them up.
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
            // Advance the channel cursor only after the day-file
            // writes succeeded.
            new_latest_ts = history.next_cursor_ts.or(new_latest_ts);
        }

        // ── Pass 2: per-thread reply fetch ───────────────────────────
        // Each thread advances independently. A failure on one thread
        // is recorded as Schema/Provider error in tracing but does NOT
        // abort the whole run nor the channel cursor.
        let parent_tss: Vec<String> = threads_cursor.keys().cloned().collect();
        for parent_ts in parent_tss {
            let prior = threads_cursor
                .get(&parent_ts)
                .and_then(|v| v.as_str())
                .map(str::to_string);
            match slack
                .thread_replies(&channel_id, &parent_ts, prior.as_deref())
                .await
            {
                Ok(page) => {
                    for msg in &page.messages {
                        // Skip the parent if we've already written it
                        // to the thread file (first call to
                        // conversations.replies returns it). We
                        // dedupe on `ts`: any message with ts ==
                        // parent_ts that we already seeded gets
                        // skipped.
                        let ts = msg
                            .get("ts")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default();
                        if ts == parent_ts && prior.is_none() {
                            // Parent already seeded above (or in a
                            // prior run that captured it). Skip.
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
                    // Leave threads_cursor[parent_ts] untouched so we
                    // retry from the same point next run.
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
}

// ── Disk helpers ────────────────────────────────────────────────────

fn append_message_to_day(
    feed_dir: &Path,
    msg: &Value,
    ts: &str,
) -> Result<u64, FeedError> {
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
/// it as YYYY-MM-DD UTC.
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
    fn validate_rejects_missing_channel() {
        let t = ChannelArchiveTemplate;
        let err = t.validate(&TemplateParams::new(json!({}))).unwrap_err();
        assert!(matches!(err, FeedError::InvalidParams(_)));
    }

    #[test]
    fn validate_rejects_empty_channel() {
        let t = ChannelArchiveTemplate;
        let err = t
            .validate(&TemplateParams::new(json!({ "channel": "  " })))
            .unwrap_err();
        assert!(matches!(err, FeedError::InvalidParams(_)));
    }

    #[test]
    fn validate_accepts_named_or_id_channel() {
        let t = ChannelArchiveTemplate;
        t.validate(&TemplateParams::new(json!({ "channel": "#design" })))
            .unwrap();
        t.validate(&TemplateParams::new(json!({ "channel": "CABCDEF" })))
            .unwrap();
    }

    #[test]
    fn ts_to_yyyy_mm_dd_parses_slack_format() {
        use chrono::TimeZone;
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
