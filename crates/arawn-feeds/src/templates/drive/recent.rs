//! `drive/recent` — personal feed: every Drive file modified in the
//! last N days. Auto-creates on `/connect google_drive` (singleton —
//! one recent feed per Drive account).
//!
//! Disk layout:
//!
//! ```text
//! drive/recent/<feed_id>/
//!   ├── meta.json                       # cursor: { latest_modified_iso }
//!   ├── 2026-05-08/
//!   │   ├── <file_id_a>.json            # DriveFile metadata snapshot
//!   │   └── <file_id_b>.json
//!   └── 2026-05-07/
//!       └── <file_id_c>.json
//! ```
//!
//! Metadata only — bodies aren't mirrored. The agent reads the
//! metadata to learn what changed; it can call `drive_read` if it
//! needs the body.
//!
//! Cursor advances to the highest `modifiedTime` observed. Drive's
//! `modifiedTime > '<iso>'` query is timestamp-grained (not
//! day-grained, unlike Slack/Gmail), so we don't need an in-template
//! dedupe window — the cursor is exact.
//!
//! Optional params:
//! - `days_back: u32` (default 7, validated 1..=90) — only used on
//!   the first run, when the cursor is null. Bounds the initial pull.

use std::path::Path;
use std::time::Instant;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};

use super::common::modified_to_yyyy_mm_dd;
use crate::clients::DriveFile;
use crate::error::FeedError;
use crate::template::{FeedTemplate, RunOutcome, TemplateCtx};
use crate::types::{FeedDefaults, RunSummary, TemplateParams};

pub struct RecentTemplate;

const NAME: &str = "drive/recent";
const DEFAULT_DAYS_BACK: i64 = 7;
const MAX_RESULTS_PER_RUN: u32 = 200;
/// Cap used when in backfill mode (cursor null + `since` present).
/// The drive adapter walks Drive's pageToken until it has this many
/// files or the result set is exhausted. See ARAWN-T-0234.
const BACKFILL_MAX_RESULTS: u32 = 5_000;

#[async_trait]
impl FeedTemplate for RecentTemplate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn validate(&self, params: &TemplateParams) -> Result<(), FeedError> {
        if let Some(v) = params.0.get("days_back") {
            let n = v.as_u64().ok_or_else(|| {
                FeedError::InvalidParams("days_back must be a non-negative integer".into())
            })?;
            if !(1..=90).contains(&n) {
                return Err(FeedError::InvalidParams(
                    "days_back must be between 1 and 90".into(),
                ));
            }
        }
        Ok(())
    }

    fn defaults(&self, _params: &TemplateParams) -> FeedDefaults {
        FeedDefaults {
            cadence: "*/30 * * * *".into(),
            initial_cursor: json!({ "latest_modified_iso": Value::Null }),
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
        let drive = ctx.clients().drive().ok_or_else(|| {
            FeedError::Auth("google drive integration not connected".into())
        })?;

        let days_back = params
            .0
            .get("days_back")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_DAYS_BACK as u64) as i64;

        // Cursor wins when present. On first run with `params.since`
        // set, that's the time floor and we hit the backfill cap to
        // cover deeper history (T-0234). Otherwise fall back to
        // `now - days_back`.
        let cursor_iso = cursor
            .get("latest_modified_iso")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());
        let params_since = params
            .0
            .get("since")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());
        let (since, max_results): (DateTime<Utc>, u32) = match (cursor_iso, params_since) {
            (Some(s), _) => (
                DateTime::parse_from_rfc3339(s)
                    .map_err(|e| FeedError::Schema(format!("bad cursor latest_modified_iso: {e}")))?
                    .with_timezone(&Utc),
                MAX_RESULTS_PER_RUN,
            ),
            (None, Some(since_iso)) => (
                DateTime::parse_from_rfc3339(since_iso)
                    .map_err(|e| FeedError::InvalidParams(format!("bad since: {e}")))?
                    .with_timezone(&Utc),
                BACKFILL_MAX_RESULTS,
            ),
            (None, None) => (Utc::now() - Duration::days(days_back), MAX_RESULTS_PER_RUN),
        };

        let files = drive.list_modified_since(since, max_results).await?;

        let mut total_items: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut new_latest: Option<String> = cursor
            .get("latest_modified_iso")
            .and_then(|v| v.as_str())
            .map(str::to_string);

        for file in &files {
            let modified = file.modified_time.as_deref();
            // Files without a modifiedTime are skipped — we can't day-
            // partition them, and Drive should always return one.
            let day = match modified_to_yyyy_mm_dd(modified) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let day_dir = feed_dir.join(&day);
            std::fs::create_dir_all(&day_dir).map_err(|e| {
                FeedError::Storage(format!("create {}: {e}", day_dir.display()))
            })?;
            let path = day_dir.join(format!("{}.json", file.id));
            let bytes = write_file_metadata(&path, file)?;
            total_items += 1;
            total_bytes += bytes;

            if new_latest
                .as_deref()
                .map(|n| modified.unwrap_or("") > n)
                .unwrap_or(true)
            {
                new_latest = modified.map(str::to_string);
            }
        }

        let new_cursor = json!({
            "latest_modified_iso": new_latest.map(Value::String).unwrap_or(Value::Null),
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

fn write_file_metadata(path: &Path, file: &DriveFile) -> Result<u64, FeedError> {
    let body = serde_json::to_vec_pretty(file)
        .map_err(|e| FeedError::Storage(format!("serialize file metadata: {e}")))?;
    let len = body.len() as u64;
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
    fn validate_default_params() {
        RecentTemplate.validate(&TemplateParams::default()).unwrap();
    }

    #[test]
    fn validate_rejects_bad_days_back() {
        let p = TemplateParams(json!({ "days_back": 0 }));
        assert!(RecentTemplate.validate(&p).is_err());
        let p = TemplateParams(json!({ "days_back": 1000 }));
        assert!(RecentTemplate.validate(&p).is_err());
    }

    #[test]
    fn defaults_use_30min_cadence() {
        let d = RecentTemplate.defaults(&TemplateParams::default());
        assert_eq!(d.cadence, "*/30 * * * *");
    }
}
