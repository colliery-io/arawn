//! `jira/assignee-tracker` — personal feed: every Jira issue
//! currently assigned to me. Auto-creates on `/connect atlassian`
//! (singleton — one assignee feed per Atlassian account).
//!
//! Lighter than `project-tracker` by design: snapshot only, no
//! comments or history logs. The personal feed answers "what's on my
//! plate"; if you want full discussion threads, point a
//! `project-tracker` at the project the issue lives in.
//!
//! Cursor: `{ latest_updated_iso }`. No per-issue cursor map — there
//! are no append-only logs to advance independently of the snapshot.
//!
//! Every snapshot carries `assigned_to_me`. Recheck: an issue reassigned
//! away from me leaves the query, so its snapshot would stay "mine"
//! forever. After the query, the open snapshots still marked mine that
//! this run did not return are rechecked with
//! `key in (...) AND assignee = currentUser()`, 50 keys per request.
//! Those still mine are kept as they are. Each of the others is fetched:
//! one that comes back is rewritten with `assigned_to_me: false`; one
//! that is not found (deleted, or no longer visible) has its snapshot
//! removed. Done snapshots are not rechecked: they are not in the brief,
//! and their number only grows.

use std::collections::HashSet;
use std::path::Path;
use std::time::Instant;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::common::{CursorState, write_issue_snapshot};
use crate::error::FeedError;
use crate::template::{FeedTemplate, RunOutcome, TemplateCtx};
use crate::types::{FeedDefaults, RunSummary, TemplateParams};

pub struct AssigneeTrackerTemplate;

const NAME: &str = "jira/assignee-tracker";
const MAX_RESULTS_PER_RUN: u32 = 100;
/// Keys per recheck request.
const RECHECK_BATCH: usize = 50;

#[async_trait]
impl FeedTemplate for AssigneeTrackerTemplate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn validate(&self, _params: &TemplateParams) -> Result<(), FeedError> {
        Ok(())
    }

    /// Singleton feed (issues assigned to the current user). Only the shared
    /// first-run `since` backfill knob applies.
    fn param_schema(&self) -> Vec<crate::param_schema::ParamSpec> {
        vec![crate::param_schema::ParamSpec::since()]
    }

    fn defaults(&self, _params: &TemplateParams) -> FeedDefaults {
        FeedDefaults {
            cadence: "*/30 * * * *".into(),
            initial_cursor: json!({
                "latest_updated_iso": Value::Null,
                "issues": {},
            }),
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
        let atlassian = ctx
            .clients()
            .atlassian()
            .ok_or_else(|| FeedError::Auth("atlassian integration not connected".into()))?;

        let mut state = CursorState::from_value(cursor);
        // First-run-only `since=` seed — see project_tracker for the
        // shared semantic. After cursor advances past `since`, it's
        // ignored.
        let effective_since = super::project_tracker::effective_since(
            state.latest_updated_iso.as_deref(),
            params.0.get("since").and_then(|v| v.as_str()),
        );
        let jql = build_jql(effective_since.as_deref());
        let issues = atlassian.jql_search(&jql, MAX_RESULTS_PER_RUN).await?;

        let mut total_items: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut new_latest = state.latest_updated_iso.clone();
        let seen: HashSet<&str> = issues.iter().map(|m| m.key.as_str()).collect();

        for meta in &issues {
            let detail = match atlassian.issue_full(&meta.key, false, false).await {
                Ok(d) => d,
                Err(FeedError::Schema(msg)) | Err(FeedError::Provider(msg)) => {
                    tracing::warn!(
                        target: "arawn::feeds",
                        feed = NAME,
                        key = %meta.key,
                        error = %msg,
                        "skipping issue"
                    );
                    continue;
                }
                Err(other) => return Err(other),
            };

            let issue_dir = feed_dir.join(&detail.meta.key);
            let snap_bytes = write_issue_snapshot(&issue_dir, &detail, Some(true))?;
            total_items += 1;
            total_bytes += snap_bytes;

            if let Some(updated) = detail.meta.updated.as_deref()
                && new_latest.as_deref().map(|n| updated > n).unwrap_or(true)
            {
                new_latest = Some(updated.to_string());
            }
        }

        // Rewrite or remove the open "mine" snapshots the query did not
        // return (see the module docs).
        let stale: Vec<String> = open_mine_snapshots(feed_dir)?
            .into_iter()
            .filter(|k| !seen.contains(k.as_str()))
            .collect();
        for batch in stale.chunks(RECHECK_BATCH) {
            let still_mine = recheck_batch(atlassian.as_ref(), feed_dir, batch).await?;
            for key in batch.iter().filter(|k| !still_mine.contains(k.as_str())) {
                let issue_dir = feed_dir.join(key);
                if !issue_dir.exists() {
                    continue; // removed by the recheck's lookup
                }
                match atlassian.issue_full(key, false, false).await {
                    Ok(detail) => {
                        total_bytes += write_issue_snapshot(&issue_dir, &detail, Some(false))?;
                        total_items += 1;
                    }
                    // 404: deleted, or I can no longer see it.
                    Err(FeedError::InvalidParams(_)) => remove_snapshot(&issue_dir)?,
                    Err(FeedError::Schema(msg)) | Err(FeedError::Provider(msg)) => {
                        tracing::warn!(
                            target: "arawn::feeds",
                            feed = NAME,
                            key = %key,
                            error = %msg,
                            "recheck: keeping issue"
                        );
                    }
                    Err(other) => return Err(other),
                }
            }
        }

        state.latest_updated_iso = new_latest;
        let status = if total_items == 0 {
            "no-new-items".to_string()
        } else {
            "ok".to_string()
        };

        Ok(RunOutcome {
            cursor: state.into_value(),
            summary: RunSummary {
                items_written: total_items,
                bytes_written: total_bytes,
                duration: started.elapsed(),
            },
            status,
        })
    }
}

/// The keys of `batch` still assigned to me. Jira rejects the whole query
/// (400) when one key no longer exists or is not visible, so on such an
/// error each key is looked up: a missing one has its snapshot removed,
/// and the query is retried once on the rest.
async fn recheck_batch(
    atlassian: &dyn crate::clients::AtlassianFeedClient,
    feed_dir: &Path,
    batch: &[String],
) -> Result<HashSet<String>, FeedError> {
    match atlassian
        .jql_search(&recheck_jql(batch), RECHECK_BATCH as u32)
        .await
    {
        Ok(found) => return Ok(found.into_iter().map(|m| m.key).collect()),
        Err(FeedError::Provider(msg)) | Err(FeedError::Schema(msg)) => {
            tracing::warn!(
                target: "arawn::feeds",
                feed = NAME,
                error = %msg,
                "recheck query failed; looking up each key"
            );
        }
        Err(other) => return Err(other),
    }
    let mut visible = Vec::new();
    for key in batch {
        match atlassian.issue_full(key, false, false).await {
            Ok(_) => visible.push(key.clone()),
            Err(FeedError::InvalidParams(_)) => remove_snapshot(&feed_dir.join(key))?,
            Err(FeedError::Schema(_)) | Err(FeedError::Provider(_)) => visible.push(key.clone()),
            Err(other) => return Err(other),
        }
    }
    if visible.is_empty() {
        return Ok(HashSet::new());
    }
    match atlassian
        .jql_search(&recheck_jql(&visible), RECHECK_BATCH as u32)
        .await
    {
        Ok(found) => Ok(found.into_iter().map(|m| m.key).collect()),
        // Still failing: keep the batch as it is until the next run.
        Err(FeedError::Provider(_)) | Err(FeedError::Schema(_)) => {
            Ok(visible.into_iter().collect())
        }
        Err(other) => Err(other),
    }
}

fn recheck_jql(keys: &[String]) -> String {
    let quoted: Vec<String> = keys.iter().map(|k| format!("\"{k}\"")).collect();
    format!(
        "key in ({}) AND assignee = currentUser()",
        quoted.join(", ")
    )
}

fn remove_snapshot(issue_dir: &Path) -> Result<(), FeedError> {
    match std::fs::remove_dir_all(issue_dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(FeedError::Storage(format!(
            "remove {}: {e}",
            issue_dir.display()
        ))),
    }
}

/// Keys of the snapshots under `feed_dir` that are mine (a snapshot from
/// before `assigned_to_me` was written counts as mine) and whose status
/// category is not done, sorted. Unreadable snapshots are skipped.
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
        let mine = v.get("assigned_to_me").and_then(Value::as_bool) != Some(false);
        let done = v
            .pointer("/fields/status/statusCategory/key")
            .and_then(Value::as_str)
            == Some("done");
        if let (true, false, Some(key)) = (mine, done, v.get("key").and_then(Value::as_str)) {
            out.push(key.to_string());
        }
    }
    out.sort();
    Ok(out)
}

fn build_jql(since: Option<&str>) -> String {
    let mut jql = "assignee = currentUser()".to_string();
    if let Some(s) = since {
        jql.push_str(&format!(" AND updated >= \"{s}\""));
    }
    jql.push_str(" ORDER BY updated ASC");
    jql
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_takes_no_params() {
        AssigneeTrackerTemplate
            .validate(&TemplateParams::default())
            .unwrap();
    }

    #[test]
    fn jql_uses_currentUser() {
        assert_eq!(
            build_jql(None),
            "assignee = currentUser() ORDER BY updated ASC"
        );
        assert_eq!(
            build_jql(Some("2026-05-08 09:00")),
            "assignee = currentUser() AND updated >= \"2026-05-08 09:00\" ORDER BY updated ASC"
        );
    }
}
