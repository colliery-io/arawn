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

use async_trait::async_trait;
use serde_json::{Value, json};

use super::common::archive_channel_with_threads;
use crate::error::FeedError;
use crate::template::{DiscoveryRow, FeedTemplate, RunOutcome, TemplateCtx};
use crate::types::{FeedDefaults, TemplateParams};

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
        let slack = ctx.clients().slack().ok_or_else(|| {
            FeedError::Auth("slack integration not connected".into())
        })?;

        let raw_channel = params
            .get_str("channel")
            .ok_or_else(|| FeedError::InvalidParams("missing `channel` param".into()))?;
        let channel_id = slack.resolve_channel(raw_channel).await?;

        archive_channel_with_threads(slack.as_ref(), &channel_id, feed_dir, cursor).await
    }

    async fn discover(
        &self,
        ctx: &TemplateCtx,
    ) -> Result<Option<Vec<DiscoveryRow>>, FeedError> {
        let slack = match ctx.clients().slack() {
            Some(c) => c,
            None => return Ok(None),
        };
        let mut channels = slack.list_channels().await?;
        // Stable, friendly order: by name.
        channels.sort_by(|a, b| a.name.cmp(&b.name));
        let rows = channels
            .into_iter()
            .map(|ch| {
                let mut tags = Vec::new();
                if ch.is_private {
                    tags.push("private");
                }
                if ch.is_dm {
                    tags.push("dm/group");
                }
                let hint = if tags.is_empty() {
                    Some(ch.id.clone())
                } else {
                    Some(format!("{}  ·  {}", ch.id, tags.join(", ")))
                };
                DiscoveryRow {
                    label: format!("#{}", ch.name),
                    hint,
                    params: json!({ "channel": ch.id }),
                }
            })
            .collect();
        Ok(Some(rows))
    }
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
}

