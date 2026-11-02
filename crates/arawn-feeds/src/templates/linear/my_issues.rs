//! `linear/my-issues` — personal feed: the Linear issues assigned to me
//! or created by me. Auto-creates when Linear is connected (singleton,
//! id `linear-mine`).
//!
//! Storage:
//!
//! ```text
//! linear/my-issues/<feed_id>/
//!   ├── meta.json                 # cursor: { latest_updated_iso }
//!   └── <issue id>/issue.json     # latest snapshot, overwrite
//! ```
//!
//! The directory is the issue's stable id, not its identifier: moving
//! an issue to another team changes `ENG-12` to `OPS-3`.
//!
//! Cursor: `{ latest_updated_iso }`, the highest `updatedAt` seen. Each
//! run asks for `updatedAt > cursor` and follows every page, so the
//! cursor only advances once the whole run has been written. A first
//! run starts from the `since` param when it is given.
//!
//! Recheck: an issue reassigned away from me that I did not create
//! leaves the query, so its snapshot would stay "assigned to me"
//! forever. After the query, every open snapshot still marked
//! `assigned_to_me` that this run did not return is fetched by id (50
//! per request) and rewritten; one that comes back no more (deleted,
//! archived, or out of the key's teams) has its snapshot removed.
//! Closed snapshots are not rechecked: they are not in the brief, and
//! rechecking them would grow without bound.

use std::collections::HashSet;
use std::path::Path;
use std::time::Instant;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::clients::LinearIssue;
use crate::error::FeedError;
use crate::template::{FeedTemplate, RunOutcome, TemplateCtx};
use crate::templates::jira::common::write_json_atomic;
use crate::types::{FeedDefaults, RunSummary, TemplateParams};

pub struct MyIssuesTemplate;

const NAME: &str = "linear/my-issues";
/// 50 issues per page; a cap so a runaway cursor cannot loop forever.
const MAX_PAGES_PER_RUN: usize = 40;
/// Ids per by-id recheck request (one page).
const RECHECK_BATCH: usize = 50;

#[async_trait]
impl FeedTemplate for MyIssuesTemplate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn validate(&self, _params: &TemplateParams) -> Result<(), FeedError> {
        Ok(())
    }

    /// Singleton feed (issues of the key's user). Only the shared
    /// first-run `since` backfill knob applies.
    fn param_schema(&self) -> Vec<crate::param_schema::ParamSpec> {
        vec![crate::param_schema::ParamSpec::since()]
    }

    fn defaults(&self, _params: &TemplateParams) -> FeedDefaults {
        FeedDefaults {
            cadence: "*/30 * * * *".into(),
            initial_cursor: json!({ "latest_updated_iso": Value::Null }),
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
        let linear = ctx
            .clients()
            .linear()
            .ok_or_else(|| FeedError::Auth("linear integration not connected".into()))?;

        let prior = cursor
            .get("latest_updated_iso")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(String::from);
        let since = effective_since(
            prior.as_deref(),
            params.0.get("since").and_then(Value::as_str),
        );

        let mut total_items: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut latest = prior;
        let mut after: Option<String> = None;
        let mut seen: HashSet<String> = HashSet::new();
        for page_no in 0.. {
            if page_no == MAX_PAGES_PER_RUN {
                tracing::warn!(
                    target: "arawn::feeds",
                    feed = NAME,
                    pages = MAX_PAGES_PER_RUN,
                    "page cap reached; the next run continues"
                );
                break;
            }
            let (viewer_id, page) = linear
                .my_issues_page(since.as_deref(), after.as_deref())
                .await?;
            for issue in &page.issues {
                total_bytes += write_snapshot(&feed_dir.join(&issue.id), issue, &viewer_id)?;
                total_items += 1;
                seen.insert(issue.id.clone());
                if latest
                    .as_deref()
                    .map(|l| is_later(&issue.updated_at, l))
                    .unwrap_or(true)
                {
                    latest = Some(issue.updated_at.clone());
                }
            }
            match page.next {
                Some(c) => after = Some(c),
                None => break,
            }
        }

        // Rewrite or remove the open "mine" snapshots the query did not
        // return (see the module docs).
        let stale: Vec<String> = open_mine_snapshots(feed_dir)?
            .into_iter()
            .filter(|id| !seen.contains(id))
            .collect();
        for batch in stale.chunks(RECHECK_BATCH) {
            let (viewer_id, issues) = linear.issues_by_ids(batch).await?;
            let mut back: HashSet<&str> = HashSet::new();
            for issue in &issues {
                total_bytes += write_snapshot(&feed_dir.join(&issue.id), issue, &viewer_id)?;
                total_items += 1;
                back.insert(issue.id.as_str());
            }
            for gone in batch.iter().filter(|id| !back.contains(id.as_str())) {
                let dir = feed_dir.join(gone);
                std::fs::remove_dir_all(&dir)
                    .map_err(|e| FeedError::Storage(format!("remove {}: {e}", dir.display())))?;
            }
        }

        Ok(RunOutcome {
            cursor: json!({ "latest_updated_iso": latest }),
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

/// Ids of the snapshots under `feed_dir` that are marked
/// `assigned_to_me` and whose state is not completed / canceled, sorted.
/// Unreadable snapshots are skipped.
fn open_mine_snapshots(feed_dir: &Path) -> Result<Vec<String>, FeedError> {
    let entries = match std::fs::read_dir(feed_dir) {
        Ok(it) => it,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(FeedError::Storage(format!(
                "read {}: {e}",
                feed_dir.display()
            )));
        }
    };
    let mut out = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| FeedError::Storage(e.to_string()))?;
        let Ok(bytes) = std::fs::read(entry.path().join("issue.json")) else {
            continue;
        };
        let Ok(v) = serde_json::from_slice::<Value>(&bytes) else {
            continue;
        };
        let mine = v.get("assigned_to_me").and_then(Value::as_bool) == Some(true);
        let closed = matches!(
            v.pointer("/state/type").and_then(Value::as_str),
            Some("completed" | "canceled")
        );
        if let (true, false, Some(id)) = (mine, closed, v.get("id").and_then(Value::as_str)) {
            out.push(id.to_string());
        }
    }
    out.sort();
    Ok(out)
}

/// The cursor when there is one; else the `since` param (RFC3339),
/// normalised to UTC. A `since` that does not parse is ignored.
fn effective_since(cursor_iso: Option<&str>, params_since: Option<&str>) -> Option<String> {
    if let Some(c) = cursor_iso {
        return Some(c.to_string());
    }
    let since = params_since.filter(|s| !s.is_empty())?;
    chrono::DateTime::parse_from_rfc3339(since)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc).to_rfc3339())
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

/// Write `<issue_dir>/issue.json`: the fields the projection and the
/// agent read, in a shape owned here rather than by Linear's schema.
fn write_snapshot(issue_dir: &Path, i: &LinearIssue, viewer_id: &str) -> Result<u64, FeedError> {
    std::fs::create_dir_all(issue_dir)
        .map_err(|e| FeedError::Storage(format!("create {}: {e}", issue_dir.display())))?;
    let person = |u: &Option<arawn_integrations::linear::models::User>| {
        u.as_ref()
            .map(|u| json!({ "id": u.id, "name": u.name, "email": u.email }))
    };
    let snapshot = json!({
        "id": i.id,
        "identifier": i.identifier,
        "title": i.title,
        "description": i.description,
        "url": i.url,
        "state": { "name": i.state.name, "type": i.state.kind },
        "priority": i.priority,
        "priority_label": i.priority_label,
        "due_date": i.due_date,
        "team": { "key": i.team.key, "name": i.team.name },
        "labels": i.labels.nodes.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(),
        "assignee": person(&i.assignee),
        "creator": person(&i.creator),
        "assigned_to_me": i.assignee.as_ref().is_some_and(|a| a.id == viewer_id),
        "created_at": i.created_at,
        "updated_at": i.updated_at,
        "comments": i.comments.nodes.iter().map(|c| json!({
            "id": c.id,
            "author": c.user.as_ref().map(|u| u.name.as_str()),
            "body": c.body,
            "created_at": c.created_at,
        })).collect::<Vec<_>>(),
    });
    let body = serde_json::to_vec_pretty(&snapshot)
        .map_err(|e| FeedError::Storage(format!("serialize issue: {e}")))?;
    let len = body.len() as u64;
    write_json_atomic(&issue_dir.join("issue.json"), &body)?;
    Ok(len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn since_prefers_the_cursor_and_normalises_the_param() {
        assert_eq!(
            effective_since(
                Some("2026-10-05T00:00:00.000Z"),
                Some("2026-01-01T00:00:00Z")
            ),
            Some("2026-10-05T00:00:00.000Z".into())
        );
        assert_eq!(
            effective_since(None, Some("2026-01-01T02:00:00+02:00")),
            Some("2026-01-01T00:00:00+00:00".into())
        );
        assert_eq!(effective_since(None, Some("yesterday")), None);
        assert_eq!(effective_since(None, None), None);
    }

    #[test]
    fn later_compares_instants_not_strings() {
        assert!(is_later(
            "2026-10-05T10:00:00.000Z",
            "2026-10-05T11:00:00+02:00"
        ));
        assert!(!is_later(
            "2026-10-05T09:00:00.000Z",
            "2026-10-05T09:00:00Z"
        ));
    }
}
