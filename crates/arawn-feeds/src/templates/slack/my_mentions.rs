//! `slack/my-mentions` — personal feed: every message anywhere in the
//! workspace that contains an `@me` mention.
//!
//! Auto-creates on `/connect slack` as `slack-<alias>-mentions`, one per
//! workspace: `workspace` names it (COLLIERY-I-0520). Backed by Slack's `search.messages` API
//! (user-token only; needs the `search:read` scope, which is in our
//! standard OAuth grant).
//!
//! Disk layout:
//!
//! ```text
//! slack/my-mentions/me/
//!   ├── meta.json                       # cursor: { my_user_id, latest_ts }
//!   ├── 2026-05-08.jsonl                # mention messages by their own Slack ts
//!   └── 2026-05-07.jsonl
//! ```
//!
//! No `threads/` subdir — a mention is a single moment; if you want
//! the surrounding thread, run a `channel-archive` feed on the
//! channel and follow the `thread_ts` field that's preserved in the
//! raw payload.
//!
//! Cursor model: `{ my_user_id, latest_ts }`. `my_user_id` resolved
//! once via `auth.test` and cached forever (the user the OAuth token
//! belongs to doesn't change). `latest_ts` is the highest mention ts
//! we've persisted; subsequent runs filter via `search.messages`'s
//! `after:YYYY-MM-DD` operator (lossy — we may re-fetch up to one
//! day's overlap, then dedupe in-template).
//!
//! Out of scope:
//!
//! - `@channel` / `@here` / `@everyone` broadcasts. A
//!   `slack/my-broadcasts` template can union those into a separate
//!   feed if anyone asks.
//! - Custom alert keywords. Same — separate template.

use std::path::Path;
use std::time::Instant;

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Value, json};

use super::workspace::{check_workspace, client, workspace_param, workspace_rows, workspace_spec};
use crate::error::FeedError;
use crate::template::{DiscoveryRow, FeedTemplate, RunOutcome, TemplateCtx};
use crate::types::{FeedDefaults, RunSummary, TemplateParams};

pub struct MyMentionsTemplate;

const NAME: &str = "slack/my-mentions";

#[async_trait]
impl FeedTemplate for MyMentionsTemplate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn validate(&self, params: &TemplateParams) -> Result<(), FeedError> {
        workspace_param(NAME, params).map(|_| ())
    }

    fn validate_with(&self, ctx: &TemplateCtx, params: &TemplateParams) -> Result<(), FeedError> {
        check_workspace(NAME, ctx, params)
    }

    /// Only the workspace: the feed is the user's own mentions there.
    fn param_schema(&self) -> Vec<crate::param_schema::ParamSpec> {
        vec![workspace_spec()]
    }

    async fn discover(&self, ctx: &TemplateCtx) -> Result<Option<Vec<DiscoveryRow>>, FeedError> {
        Ok(Some(workspace_rows(ctx)))
    }

    fn defaults(&self, _params: &TemplateParams) -> FeedDefaults {
        FeedDefaults {
            cadence: "*/15 * * * *".into(),
            initial_cursor: json!({ "my_user_id": Value::Null, "latest_ts": Value::Null }),
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
        let slack = client(NAME, ctx, params)?;

        // ── 1. Resolve our user_id (cache-or-call) ───────────────────
        let my_user_id = match cursor.get("my_user_id").and_then(|v| v.as_str()) {
            Some(id) if !id.is_empty() => id.to_string(),
            _ => slack.auth_test().await?.user_id,
        };

        // ── 2. Build query + run search ──────────────────────────────
        let query = format!("<@{my_user_id}>");
        let prior_latest = cursor
            .get("latest_ts")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let page = slack
            .search_messages(&query, prior_latest.as_deref())
            .await?;

        // ── 3. Dedupe + write ────────────────────────────────────────
        // Slack's `after:YYYY-MM-DD` is day-grained, so we may see
        // messages we've already persisted from earlier on the same
        // day. Filter by exact ts > prior_latest.
        let mut total_items: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut new_latest_ts: Option<String> = prior_latest.clone();

        for msg in &page.messages {
            let ts = match msg.get("ts").and_then(|v| v.as_str()) {
                Some(t) => t,
                None => continue, // search results occasionally lack ts; skip
            };
            if let Some(prev) = prior_latest.as_deref()
                && ts <= prev
            {
                continue;
            }
            let bytes = append_message_to_day(feed_dir, msg, ts)?;
            total_items += 1;
            total_bytes += bytes;
            if new_latest_ts.as_deref().map(|n| ts > n).unwrap_or(true) {
                new_latest_ts = Some(ts.to_string());
            }
        }

        let new_cursor = json!({
            "my_user_id": my_user_id,
            "latest_ts": new_latest_ts.map(Value::String).unwrap_or(Value::Null),
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

fn append_message_to_day(feed_dir: &Path, msg: &Value, ts: &str) -> Result<u64, FeedError> {
    let day = ts_to_yyyy_mm_dd(ts)?;
    let path = feed_dir.join(format!("{day}.jsonl"));
    use std::io::Write;
    let line = serde_json::to_string(msg)
        .map_err(|e| FeedError::Storage(format!("serialize message: {e}")))?;
    let bytes = format!("{line}\n");
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| FeedError::Storage(format!("open {}: {e}", path.display())))?;
    f.write_all(bytes.as_bytes())
        .map_err(|e| FeedError::Storage(format!("append: {e}")))?;
    Ok(bytes.len() as u64)
}

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

    #[test]
    fn validate_needs_a_workspace() {
        let t = MyMentionsTemplate;
        let err = t.validate(&TemplateParams::default()).unwrap_err();
        assert!(err.to_string().contains("`workspace`"), "{err}");
        t.validate(&TemplateParams::new(json!({ "workspace": "acme" })))
            .unwrap();
        assert!(
            t.validate(&TemplateParams::new(json!({ "workspace": "Acme Corp" })))
                .is_err()
        );
    }

    #[test]
    fn defaults_provide_cursor_with_null_user_id() {
        let t = MyMentionsTemplate;
        let d = t.defaults(&TemplateParams::default());
        assert_eq!(d.cadence, "*/15 * * * *");
        assert!(d.initial_cursor["my_user_id"].is_null());
        assert!(d.initial_cursor["latest_ts"].is_null());
    }
}
