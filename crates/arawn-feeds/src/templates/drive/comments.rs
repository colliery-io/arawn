//! `drive/comments` — the open comment threads that are for me on the
//! Docs, Sheets and Slides I have touched (COLLIERY-I-0533).
//!
//! Storage:
//!
//! ```text
//! drive/comments/<feed_id>/
//!   ├── meta.json                      # cursor (below)
//!   └── <file id>/<comment id>.json    # one snapshot per thread for me
//! ```
//!
//! Discovery: `files.list` for Docs, Sheets and Slides not in the trash,
//! with `modifiedTime > T or viewedByMeTime > T`, where T = now −
//! `lookback_days` (default 14, 1–90). `extra_file_ids` adds files by id
//! (for a doc found another way, e.g. a Gmail mention email).
//!
//! Filter: [`for_me`] — not resolved, not deleted, and assigned to me,
//! mentioning me, or on a doc I own where the last message is not mine.
//! A thread that stops being for me (resolved, deleted, answered by me)
//! has its snapshot removed, so the projection drops its row.
//!
//! Cursor: `{ docs: { <file id>: <latest comment modifiedTime> },
//! lookback_floor }`. Each doc is polled with `startModifiedTime` = its
//! entry (all comments when it has none). The API treats the time as a
//! minimum, so the newest thread comes back each run; an unchanged
//! snapshot is not rewritten. A doc that leaves the window is dropped
//! from the cursor; its snapshots stay.

use std::path::Path;
use std::time::Instant;

use arawn_integrations::drive::comments::{ForMe, comment_link, for_me};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde_json::{Map, Value, json};

use super::common::sanitize_path_component;
use crate::clients::{DocComment, DocMeta};
use crate::error::FeedError;
use crate::template::{FeedTemplate, RunOutcome, TemplateCtx};
use crate::templates::jira::common::write_json_atomic;
use crate::types::{FeedDefaults, RunSummary, TemplateParams};

pub struct CommentsTemplate;

const NAME: &str = "drive/comments";
const DEFAULT_LOOKBACK_DAYS: u64 = 14;

#[async_trait]
impl FeedTemplate for CommentsTemplate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn validate(&self, params: &TemplateParams) -> Result<(), FeedError> {
        if let Some(v) = params.0.get("lookback_days") {
            let n = v.as_u64().ok_or_else(|| {
                FeedError::InvalidParams("lookback_days must be a non-negative integer".into())
            })?;
            if !(1..=90).contains(&n) {
                return Err(FeedError::InvalidParams(
                    "lookback_days must be between 1 and 90".into(),
                ));
            }
        }
        if let Some(v) = params.0.get("extra_file_ids") {
            let ok = v.as_array().is_some_and(|a| {
                a.iter()
                    .all(|id| id.as_str().is_some_and(|s| !s.is_empty()))
            });
            if !ok {
                return Err(FeedError::InvalidParams(
                    "extra_file_ids must be a list of Drive file ids".into(),
                ));
            }
        }
        Ok(())
    }

    fn param_schema(&self) -> Vec<crate::param_schema::ParamSpec> {
        use crate::param_schema::{ParamKind, ParamSpec};
        vec![
            ParamSpec::optional(
                "lookback_days",
                "Lookback days",
                ParamKind::Int,
                json!(DEFAULT_LOOKBACK_DAYS),
                "Poll the docs I modified or opened in the last N days (1–90).",
            ),
            ParamSpec::optional_no_default(
                "extra_file_ids",
                "Extra file ids",
                ParamKind::List,
                "Drive file ids to poll as well, even when not in the lookback.",
            ),
        ]
    }

    fn defaults(&self, _params: &TemplateParams) -> FeedDefaults {
        FeedDefaults {
            cadence: "*/30 * * * *".into(),
            initial_cursor: json!({ "docs": {}, "lookback_floor": Value::Null }),
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
        let drive = ctx
            .clients()
            .drive()
            .ok_or_else(|| FeedError::Auth("google drive integration not connected".into()))?;

        let lookback = params
            .0
            .get("lookback_days")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_LOOKBACK_DAYS)
            .clamp(1, 90) as i64;
        let floor = Utc::now() - Duration::days(lookback);

        let me = drive.about_me().await?;
        let mut docs = drive.list_touched_docs(floor).await?;
        let extra = params
            .0
            .get("extra_file_ids")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>())
            .unwrap_or_default();
        for id in extra {
            if docs.iter().any(|d| d.id == id) {
                continue;
            }
            match drive.get_doc(id).await? {
                Some(d) => docs.push(d),
                None => tracing::warn!(
                    target: "arawn::feeds",
                    feed = NAME,
                    file_id = id,
                    "extra file id not found; skipped"
                ),
            }
        }

        let prior = cursor
            .get("docs")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let mut next = Map::new();
        let mut total_items: u64 = 0;
        let mut total_bytes: u64 = 0;
        for doc in &docs {
            let start = prior
                .get(&doc.id)
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty());
            let comments = drive.list_comments(&doc.id, start).await?;
            let mut latest = start.map(String::from);
            let doc_dir = feed_dir.join(sanitize_path_component(&doc.id));
            for c in &comments {
                if let Some(m) = c.modified_time.as_deref()
                    && latest.as_deref().map(|l| is_later(m, l)).unwrap_or(true)
                {
                    latest = Some(m.to_string());
                }
                let path = doc_dir.join(format!("{}.json", sanitize_path_component(&c.id)));
                match for_me(c, &me, doc.owned_by_me) {
                    Some(reason) => {
                        if let Some(n) = write_snapshot(&path, doc, c, reason)? {
                            total_items += 1;
                            total_bytes += n;
                        }
                    }
                    None if path.exists() => {
                        std::fs::remove_file(&path).map_err(|e| {
                            FeedError::Storage(format!("remove {}: {e}", path.display()))
                        })?;
                        // Only succeeds when the doc has no snapshot left.
                        let _ = std::fs::remove_dir(&doc_dir);
                        total_items += 1;
                    }
                    None => {}
                }
            }
            next.insert(
                doc.id.clone(),
                latest.map(Value::String).unwrap_or(Value::Null),
            );
        }

        Ok(RunOutcome {
            cursor: json!({ "docs": next, "lookback_floor": floor.to_rfc3339() }),
            summary: RunSummary {
                items_written: total_items,
                bytes_written: total_bytes,
                duration: started.elapsed(),
            },
            status: if total_items == 0 {
                "no-new-items".into()
            } else {
                "ok".into()
            },
        })
    }
}

/// `a` is after `b`, both RFC3339. Falls back to string order when one
/// does not parse.
fn is_later(a: &str, b: &str) -> bool {
    match (
        chrono::DateTime::parse_from_rfc3339(a),
        chrono::DateTime::parse_from_rfc3339(b),
    ) {
        (Ok(a), Ok(b)) => a > b,
        _ => a > b,
    }
}

/// Write the snapshot of one thread. `None` when the file already holds
/// the same bytes (the boundary thread comes back every run).
fn write_snapshot(
    path: &Path,
    doc: &DocMeta,
    c: &DocComment,
    reason: ForMe,
) -> Result<Option<u64>, FeedError> {
    let last = c.last_reply();
    let snapshot = json!({
        "file_id": doc.id,
        "comment_id": c.id,
        "doc_title": doc.name,
        "mime_type": doc.mime_type,
        "owned_by_me": doc.owned_by_me,
        "link": doc.web_view_link.as_deref().map(|l| comment_link(l, &c.id)),
        "reason": reason.as_str(),
        "author": c.author,
        "content": c.content,
        "quoted": c.quoted,
        "assignee": c.assignee_email,
        "created_time": c.created_time,
        "modified_time": c.modified_time,
        "reply_count": c.live_replies().count(),
        "last_reply_author": last.and_then(|r| r.author.as_deref()),
        "last_reply": last.map(|r| r.content.as_str()),
    });
    let body = serde_json::to_vec_pretty(&snapshot)
        .map_err(|e| FeedError::Storage(format!("serialize comment: {e}")))?;
    if std::fs::read(path).is_ok_and(|old| old == body) {
        return Ok(None);
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| FeedError::Storage(format!("create {}: {e}", dir.display())))?;
    }
    write_json_atomic(path, &body)?;
    Ok(Some(body.len() as u64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_checks_lookback_and_extra_ids() {
        CommentsTemplate
            .validate(&TemplateParams::default())
            .unwrap();
        let ok = TemplateParams(json!({ "lookback_days": 30, "extra_file_ids": ["f1"] }));
        CommentsTemplate.validate(&ok).unwrap();
        for bad in [
            json!({ "lookback_days": 0 }),
            json!({ "lookback_days": 91 }),
            json!({ "extra_file_ids": "f1" }),
            json!({ "extra_file_ids": [""] }),
        ] {
            assert!(CommentsTemplate.validate(&TemplateParams(bad)).is_err());
        }
    }

    #[test]
    fn defaults_use_30min_cadence() {
        let d = CommentsTemplate.defaults(&TemplateParams::default());
        assert_eq!(d.cadence, "*/30 * * * *");
        assert_eq!(d.initial_cursor["docs"], json!({}));
    }
}
