//! `slack/channel-archive` — append every message in one Slack channel
//! to JSONL, time-partitioned by day.
//!
//! Disk layout:
//!
//! ```text
//! slack/channel-archive/<feed_id>/
//!   ├── meta.json            # cursor: { "latest_ts": "..." }
//!   ├── 2026-05-08.jsonl     # one Slack message per line, append-only
//!   ├── 2026-05-07.jsonl
//!   └── ...
//! ```
//!
//! Cursor model: persists Slack `latest_ts` (the highest message ts seen
//! so far). Subsequent runs ask for messages with `oldest = latest_ts`
//! so we only fetch new content. First run gets the last 24h.
//!
//! Each line is the raw API payload Slack returned — preserves full
//! fidelity and lets the agent introspect any field via grep / jq.

use std::path::Path;
use std::time::Instant;

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Value, json};

use crate::clients::SlackHistoryPage;
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
            initial_cursor: Value::Null,
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

        let oldest_ts = cursor.get("latest_ts").and_then(|v| v.as_str()).map(str::to_string);
        let page = slack
            .channel_history(&channel_id, oldest_ts.as_deref())
            .await?;

        let SlackHistoryPage {
            messages,
            next_cursor_ts,
        } = page;

        if messages.is_empty() {
            // Cursor unchanged; status reflects the no-op.
            return Ok(RunOutcome {
                cursor: cursor.clone(),
                summary: RunSummary {
                    items_written: 0,
                    bytes_written: 0,
                    duration: started.elapsed(),
                },
                status: "no-new-items".into(),
            });
        }

        let bytes = append_messages_partitioned(feed_dir, &messages)?;
        let new_cursor = match next_cursor_ts {
            Some(ts) => json!({ "latest_ts": ts }),
            // Provider didn't suggest one (rare); fall back to the
            // newest message's `ts`, else preserve prior cursor.
            None => match newest_ts(&messages) {
                Some(ts) => json!({ "latest_ts": ts }),
                None => cursor.clone(),
            },
        };

        Ok(RunOutcome {
            cursor: new_cursor,
            summary: RunSummary {
                items_written: messages.len() as u64,
                bytes_written: bytes,
                duration: started.elapsed(),
            },
            status: "ok".into(),
        })
    }
}

/// Append each message to the JSONL file for the day its `ts` falls on.
/// Returns total bytes written across all files touched.
fn append_messages_partitioned(
    feed_dir: &Path,
    messages: &[Value],
) -> Result<u64, FeedError> {
    use std::io::Write;
    let mut total = 0u64;
    let mut files: std::collections::HashMap<String, std::fs::File> = Default::default();

    for msg in messages {
        let ts = msg.get("ts").and_then(|v| v.as_str()).ok_or_else(|| {
            FeedError::Schema("slack message missing `ts` string field".into())
        })?;
        let day = ts_to_yyyy_mm_dd(ts)?;
        let entry = files.entry(day.clone());
        let file = match entry {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(v) => {
                let path = feed_dir.join(format!("{day}.jsonl"));
                let f = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .map_err(|e| {
                        FeedError::Storage(format!("open {}: {e}", path.display()))
                    })?;
                v.insert(f)
            }
        };

        let line = serde_json::to_string(msg)
            .map_err(|e| FeedError::Storage(format!("serialize message: {e}")))?;
        let bytes = format!("{line}\n");
        file.write_all(bytes.as_bytes())
            .map_err(|e| FeedError::Storage(format!("append: {e}")))?;
        total += bytes.len() as u64;
    }
    Ok(total)
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

fn newest_ts(messages: &[Value]) -> Option<String> {
    messages
        .iter()
        .filter_map(|m| m.get("ts").and_then(|v| v.as_str()))
        .max()
        .map(str::to_string)
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
        // Round-trip via chrono so we're testing the parse logic, not
        // hand-computed unix epoch math.
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
}
