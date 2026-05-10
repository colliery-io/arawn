//! `jira/project-tracker` — issues + comments + history for a Jira
//! project.
//!
//! Required param:
//! - `project: string` — Jira project key (e.g. `"ENG"`) or id. The
//!   adapter resolves the param to an id at registration time so a
//!   typo fails fast.
//!
//! Storage: see [`super::common`].
//!
//! Cursor: shared `CursorState` — feed-level `latest_updated_iso`
//! plus a per-issue `{ last_comment_id, last_history_id }` map.

use std::path::Path;
use std::time::Instant;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::common::{
    CursorState, append_logs, write_issue_snapshot,
};
use crate::error::FeedError;
use crate::template::{DiscoveryRow, FeedTemplate, RunOutcome, TemplateCtx};
use crate::types::{FeedDefaults, RunSummary, TemplateParams};

pub struct ProjectTrackerTemplate;

const NAME: &str = "jira/project-tracker";
const MAX_RESULTS_PER_RUN: u32 = 100;

#[async_trait]
impl FeedTemplate for ProjectTrackerTemplate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn validate(&self, params: &TemplateParams) -> Result<(), FeedError> {
        let project = params
            .0
            .get("project")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                FeedError::InvalidParams("missing required param: project".into())
            })?;
        if project.trim().is_empty() {
            return Err(FeedError::InvalidParams(
                "project must not be empty".into(),
            ));
        }
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
        let project = params
            .0
            .get("project")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FeedError::InvalidParams("missing project".into()))?
            .to_string();

        let mut state = CursorState::from_value(cursor);
        // First-run-only `since=` seed: when the cursor is null and
        // `params.since` is set, fold it into the JQL `updated >=`
        // clause instead of letting the query drop the time floor
        // entirely. After cursor advances past `since`, params.since
        // is ignored. See ARAWN-T-0233.
        let effective_since = effective_since(
            state.latest_updated_iso.as_deref(),
            params.0.get("since").and_then(|v| v.as_str()),
        );
        let jql = build_jql(&project, effective_since.as_deref());
        let issues = atlassian.jql_search(&jql, MAX_RESULTS_PER_RUN).await?;

        let mut total_items: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut new_latest = state.latest_updated_iso.clone();

        for meta in &issues {
            // Per-issue resilience: a single failed issue doesn't
            // poison the rest of the run. Auth + rate-limit propagate
            // so the runtime can back off.
            let detail = match atlassian.issue_full(&meta.key, true, true).await {
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
            let prior = state
                .issues
                .get(&detail.meta.key)
                .cloned()
                .unwrap_or_default();
            let outcome = append_logs(&issue_dir, &detail, prior)?;
            state
                .issues
                .insert(detail.meta.key.clone(), outcome.cursor);

            total_items += 1;
            total_bytes += snap_bytes + outcome.bytes_written;

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

    async fn discover(
        &self,
        ctx: &TemplateCtx,
    ) -> Result<Option<Vec<DiscoveryRow>>, FeedError> {
        let atlassian = match ctx.clients().atlassian() {
            Some(c) => c,
            None => return Ok(None),
        };
        let mut projects = atlassian.list_jira_projects().await?;
        projects.sort_by(|a, b| a.key.cmp(&b.key));
        let rows = projects
            .into_iter()
            .map(|p| DiscoveryRow {
                label: format!("{}  —  {}", p.key, p.name),
                hint: Some(format!("id {}", p.id)),
                params: json!({ "project": p.key }),
            })
            .collect();
        Ok(Some(rows))
    }
}

/// Resolve the JQL time-floor for this run.
///
/// - Cursor present → use it (cursor wins).
/// - Cursor null + `since` (RFC3339) → convert to JQL's `YYYY-MM-DD HH:mm` form.
/// - Neither → None (no time clause; full table scan, capped at 100).
///
/// Uses the same shape as `assignee_tracker::effective_since` —
/// duplicated rather than shared to keep the templates' time-format
/// concerns local. If a third Jira template lands, hoist into a
/// shared helper.
pub(super) fn effective_since(cursor_iso: Option<&str>, params_since: Option<&str>) -> Option<String> {
    if let Some(prior) = cursor_iso
        && !prior.is_empty()
    {
        return Some(prior.to_string());
    }
    let since = params_since.filter(|s| !s.is_empty())?;
    chrono::DateTime::parse_from_rfc3339(since)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc).format("%Y-%m-%d %H:%M").to_string())
}

fn build_jql(project: &str, since: Option<&str>) -> String {
    // Project keys are uppercase letters/numbers — safe to inline
    // without quoting. ORDER BY ensures deterministic cursor advance.
    let mut jql = format!("project = {project}");
    if let Some(s) = since {
        // Jira's JQL accepts ISO-like timestamps quoted with `"`.
        jql.push_str(&format!(" AND updated >= \"{s}\""));
    }
    jql.push_str(" ORDER BY updated ASC");
    jql
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_requires_project() {
        assert!(ProjectTrackerTemplate
            .validate(&TemplateParams::default())
            .is_err());
        let p = TemplateParams(json!({ "project": "" }));
        assert!(ProjectTrackerTemplate.validate(&p).is_err());
        let p = TemplateParams(json!({ "project": "ENG" }));
        ProjectTrackerTemplate.validate(&p).unwrap();
    }

    #[test]
    fn effective_since_prefers_cursor_then_falls_back_to_params() {
        // Cursor wins when present.
        let cur = Some("2026-05-01T12:00:00.000+0000");
        let p = Some("2026-01-01T00:00:00+00:00");
        assert_eq!(effective_since(cur, p), Some(cur.unwrap().to_string()));

        // Cursor null + since set → since converted to JQL form.
        let p = Some("2026-01-01T00:00:00+00:00");
        assert_eq!(
            effective_since(None, p),
            Some("2026-01-01 00:00".to_string())
        );

        // Both null → None.
        assert_eq!(effective_since(None, None), None);

        // Garbage since (not RFC3339) → None.
        assert_eq!(effective_since(None, Some("yesterday")), None);

        // Empty cursor falls through to since.
        let p = Some("2026-01-01T00:00:00+00:00");
        assert_eq!(effective_since(Some(""), p), Some("2026-01-01 00:00".into()));
    }

    #[test]
    fn jql_includes_since_when_present() {
        assert_eq!(
            build_jql("ENG", None),
            "project = ENG ORDER BY updated ASC"
        );
        assert_eq!(
            build_jql("ENG", Some("2026-05-08 09:00")),
            "project = ENG AND updated >= \"2026-05-08 09:00\" ORDER BY updated ASC"
        );
    }
}
