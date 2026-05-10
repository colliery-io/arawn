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

#[async_trait]
impl FeedTemplate for AssigneeTrackerTemplate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn validate(&self, _params: &TemplateParams) -> Result<(), FeedError> {
        Ok(())
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
        let atlassian = ctx.clients().atlassian().ok_or_else(|| {
            FeedError::Auth("atlassian integration not connected".into())
        })?;

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
            let snap_bytes = write_issue_snapshot(&issue_dir, &detail)?;
            total_items += 1;
            total_bytes += snap_bytes;

            if let Some(updated) = detail.meta.updated.as_deref()
                && new_latest.as_deref().map(|n| updated > n).unwrap_or(true) {
                    new_latest = Some(updated.to_string());
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
