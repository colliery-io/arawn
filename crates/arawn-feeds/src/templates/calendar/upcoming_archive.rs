//! `calendar/upcoming-archive` — rolling snapshot of every event on a
//! Google Calendar between now and `window_days` ahead.
//!
//! Disk layout (per feed instance):
//!
//! ```text
//! calendar/upcoming-archive/<feed_id>/
//!   ├── meta.json                       # cursor: { last_synced_at }
//!   └── events/
//!       ├── <event_id>.json             # current state of the event
//!       └── ...
//! ```
//!
//! Storage model:
//!
//! - One file per `event_id`, **overwritten on update**. Calendar
//!   events are inherently mutable (move, retitle, cancel) — what
//!   matters to an agent is "what's on my calendar now," not "what was
//!   on my calendar two hours ago." If history is needed later we'll
//!   add an append-only `changelog.jsonl` alongside; for v1 the
//!   current-state shape is what's useful.
//! - Cancelled events keep their file (status field is preserved from
//!   the API payload) so they don't silently disappear from the
//!   archive between runs. They naturally fall out of the next sweep
//!   once they slide past `time_min`.
//! - No date partitioning. Events are keyed by id, not by occurrence
//!   day, because a recurring event expanded into 14 instances would
//!   otherwise scatter across 14 directories.
//!
//! Cursor is informational only (`last_synced_at`); each run does a
//! full window fetch. At a 30-min cadence with O(50 events) per
//! window, that's well under any quota and avoids the syncToken
//! expiration / 410-fallback complexity.
//!
//! Params:
//! - `calendar_id` (optional, default `"primary"`)
//! - `window_days` (optional, default `7`)

use std::path::Path;
use std::time::Instant;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde_json::{Value, json};

use crate::error::FeedError;
use crate::template::{FeedTemplate, RunOutcome, TemplateCtx};
use crate::types::{FeedDefaults, RunSummary, TemplateParams};

pub struct UpcomingArchiveTemplate;

const NAME: &str = "calendar/upcoming-archive";
const DEFAULT_CALENDAR_ID: &str = "primary";
const DEFAULT_WINDOW_DAYS: i64 = 7;

#[async_trait]
impl FeedTemplate for UpcomingArchiveTemplate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn validate(&self, params: &TemplateParams) -> Result<(), FeedError> {
        // Both params are optional. If `window_days` is set it must be
        // a positive integer ≤ 60 (we don't want runaway-window feeds).
        if let Some(v) = params.0.get("window_days") {
            let n = v.as_i64().ok_or_else(|| {
                FeedError::InvalidParams("window_days must be an integer".into())
            })?;
            if !(1..=60).contains(&n) {
                return Err(FeedError::InvalidParams(
                    "window_days must be between 1 and 60".into(),
                ));
            }
        }
        if let Some(v) = params.0.get("calendar_id")
            && !v.is_string()
        {
            return Err(FeedError::InvalidParams(
                "calendar_id must be a string".into(),
            ));
        }
        Ok(())
    }

    fn defaults(&self, _params: &TemplateParams) -> FeedDefaults {
        FeedDefaults {
            // Calendar changes far less often than chat. 30 min is a
            // good balance — fast enough to surface fresh invites,
            // slow enough to be polite.
            cadence: "*/30 * * * *".into(),
            initial_cursor: json!({ "last_synced_at": Value::Null }),
        }
    }

    async fn run(
        &self,
        ctx: &TemplateCtx,
        params: &TemplateParams,
        feed_dir: &Path,
        _cursor: &Value,
    ) -> Result<RunOutcome, FeedError> {
        let started = Instant::now();
        let calendar = ctx.clients().calendar().ok_or_else(|| {
            FeedError::Auth("google calendar integration not connected".into())
        })?;

        let calendar_id = params
            .0
            .get("calendar_id")
            .and_then(|v| v.as_str())
            .unwrap_or(DEFAULT_CALENDAR_ID)
            .to_string();
        let window_days = params
            .0
            .get("window_days")
            .and_then(|v| v.as_i64())
            .unwrap_or(DEFAULT_WINDOW_DAYS);

        let now = Utc::now();
        let end = now + Duration::days(window_days);

        let events = calendar.list_events(&calendar_id, now, end).await?;

        let events_dir = feed_dir.join("events");
        std::fs::create_dir_all(&events_dir).map_err(|e| {
            FeedError::Storage(format!("create events dir: {e}"))
        })?;

        let mut total_items: u64 = 0;
        let mut total_bytes: u64 = 0;
        for event in &events {
            let id = match event.get("id").and_then(|v| v.as_str()) {
                Some(s) if !s.is_empty() => s,
                _ => continue, // never expected from the API; skip defensively
            };
            // event ids are URL-safe slugs but contain '@' for some
            // calendars — sanitize for filesystem safety.
            let safe = sanitize_event_id(id);
            let path = events_dir.join(format!("{safe}.json"));
            let bytes = write_event_file(&path, event)?;
            total_items += 1;
            total_bytes += bytes;
        }

        let new_cursor = json!({ "last_synced_at": now.to_rfc3339() });

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

fn sanitize_event_id(id: &str) -> String {
    id.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
            _ => '_',
        })
        .collect()
}

fn write_event_file(path: &Path, event: &Value) -> Result<u64, FeedError> {
    let body = serde_json::to_vec_pretty(event)
        .map_err(|e| FeedError::Storage(format!("serialize event: {e}")))?;
    let len = body.len() as u64;
    // Atomic write via sibling temp file rename — survives a crash
    // mid-write without leaving a half-written event file.
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
    fn validate_accepts_default_params() {
        UpcomingArchiveTemplate
            .validate(&TemplateParams::default())
            .unwrap();
    }

    #[test]
    fn validate_rejects_bad_window_days() {
        let params = TemplateParams(json!({ "window_days": 0 }));
        assert!(UpcomingArchiveTemplate.validate(&params).is_err());
        let params = TemplateParams(json!({ "window_days": 1000 }));
        assert!(UpcomingArchiveTemplate.validate(&params).is_err());
        let params = TemplateParams(json!({ "window_days": "seven" }));
        assert!(UpcomingArchiveTemplate.validate(&params).is_err());
    }

    #[test]
    fn defaults_use_30min_cadence() {
        let d = UpcomingArchiveTemplate.defaults(&TemplateParams::default());
        assert_eq!(d.cadence, "*/30 * * * *");
    }

    #[test]
    fn sanitize_keeps_safe_chars() {
        assert_eq!(sanitize_event_id("abc123"), "abc123");
        assert_eq!(sanitize_event_id("a-b_c"), "a-b_c");
        assert_eq!(
            sanitize_event_id("foo@google.com"),
            "foo_google_com"
        );
    }
}
