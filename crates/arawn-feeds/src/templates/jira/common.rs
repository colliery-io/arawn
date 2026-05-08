//! Shared Jira-template helpers.
//!
//! Both Jira templates (`project-tracker`, `assignee-tracker`) follow
//! the same skeleton: JQL search → for each issue, write a snapshot
//! and (optionally) append-log new comments / changelog entries.
//! The single difference is the JQL clause that scopes the issues.
//!
//! Storage shapes:
//!
//! ```text
//! jira/project-tracker/<feed_id>/
//!   ├── meta.json                       # cursor (see CursorState)
//!   └── <ISSUE-KEY>/
//!       ├── issue.json                  # latest snapshot, overwrite
//!       ├── comments.jsonl              # append-only, deduped by id
//!       └── history.jsonl               # append-only, deduped by id
//!
//! jira/assignee-tracker/<feed_id>/
//!   ├── meta.json                       # cursor: { latest_updated_iso }
//!   └── <ISSUE-KEY>/
//!       └── issue.json                  # snapshot only, overwrite
//! ```
//!
//! Cursor:
//!
//! ```json
//! {
//!   "latest_updated_iso": "2026-05-08T09:00:00.000+0000",
//!   "issues": {
//!     "ENG-1": { "last_comment_id": "10042", "last_history_id": "1180" }
//!   }
//! }
//! ```
//!
//! Per-issue cursors are only used by `project-tracker`; the personal
//! `assignee-tracker` feed only carries `latest_updated_iso`.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::clients::JiraIssueDetail;
use crate::error::FeedError;

/// Per-issue cursor state. Only meaningful for templates that
/// maintain append-only logs (project-tracker).
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PerIssueCursor {
    /// Highest comment id observed for this issue. Comparison is
    /// lexicographic on the id string — Jira ids are numeric strings,
    /// so that works as long as we pad to a common width or compare as
    /// numbers. We compare numerically.
    pub last_comment_id: Option<String>,
    /// Highest changelog history id observed.
    pub last_history_id: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CursorState {
    /// Highest `updated` timestamp observed across all issues this
    /// feed has seen. Used as a JQL `updated >=` clause on next run.
    pub latest_updated_iso: Option<String>,
    /// Per-issue append-log cursors. `assignee-tracker` leaves this
    /// empty.
    #[serde(default)]
    pub issues: BTreeMap<String, PerIssueCursor>,
}

impl CursorState {
    pub fn from_value(v: &Value) -> Self {
        serde_json::from_value(v.clone()).unwrap_or_default()
    }
    pub fn into_value(self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

/// Atomic-rename write of a JSON snapshot to `path`.
pub fn write_json_atomic(path: &Path, body: &[u8]) -> Result<(), FeedError> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, body)
        .map_err(|e| FeedError::Storage(format!("write {}: {e}", tmp.display())))?;
    std::fs::rename(&tmp, path)
        .map_err(|e| FeedError::Storage(format!("rename {}: {e}", path.display())))?;
    Ok(())
}

/// Append a single JSON-serializable item as one line to `path`.
/// Created if missing. Does not deduplicate — caller filters first.
pub fn append_jsonl(path: &Path, line: &Value) -> Result<u64, FeedError> {
    use std::io::Write;
    let body = serde_json::to_string(line)
        .map_err(|e| FeedError::Storage(format!("serialize line: {e}")))?;
    let formatted = format!("{body}\n");
    let bytes = formatted.as_bytes();
    let len = bytes.len() as u64;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            FeedError::Storage(format!("create {}: {e}", parent.display()))
        })?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| FeedError::Storage(format!("open {}: {e}", path.display())))?;
    f.write_all(bytes)
        .map_err(|e| FeedError::Storage(format!("append {}: {e}", path.display())))?;
    Ok(len)
}

/// Result of writing one issue's snapshot + (optional) logs.
pub struct IssueWriteOutcome {
    pub bytes_written: u64,
    /// Updated per-issue cursor state (advances on every successful
    /// log append).
    pub cursor: PerIssueCursor,
}

/// Write `<issue_dir>/issue.json` (overwrite).
pub fn write_issue_snapshot(
    issue_dir: &Path,
    detail: &JiraIssueDetail,
) -> Result<u64, FeedError> {
    std::fs::create_dir_all(issue_dir).map_err(|e| {
        FeedError::Storage(format!("create {}: {e}", issue_dir.display()))
    })?;
    let path = issue_dir.join("issue.json");
    let snapshot = serde_json::json!({
        "key": detail.meta.key,
        "id": detail.meta.id,
        "updated": detail.meta.updated,
        "summary": detail.meta.summary,
        "fields": detail.fields,
    });
    let body = serde_json::to_vec_pretty(&snapshot)
        .map_err(|e| FeedError::Storage(format!("serialize issue: {e}")))?;
    let len = body.len() as u64;
    write_json_atomic(&path, &body)?;
    Ok(len)
}

/// Write any new comments + changelog entries to per-issue jsonl
/// files, advancing the per-issue cursor as it goes.
///
/// New entries are those whose id is greater than `prior.last_*_id`
/// (numeric comparison). When the prior cursor is `None`, every
/// returned entry is "new" — first run.
pub fn append_logs(
    issue_dir: &Path,
    detail: &JiraIssueDetail,
    prior: PerIssueCursor,
) -> Result<IssueWriteOutcome, FeedError> {
    let mut cursor = prior;
    let mut bytes_written: u64 = 0;

    if let Some(comments) = detail.comments.as_ref() {
        let path = issue_dir.join("comments.jsonl");
        let prior_id = parse_id(cursor.last_comment_id.as_deref());
        let mut highest = prior_id;
        let mut sorted: Vec<&Value> = comments.iter().collect();
        sorted.sort_by_key(|c| parse_id(c.get("id").and_then(|v| v.as_str())));
        for c in sorted {
            let id = parse_id(c.get("id").and_then(|v| v.as_str()));
            if id <= prior_id {
                continue;
            }
            bytes_written += append_jsonl(&path, c)?;
            if id > highest {
                highest = id;
            }
        }
        if highest != prior_id {
            cursor.last_comment_id = highest.map(|n| n.to_string());
        }
    }

    if let Some(histories) = detail.changelog.as_ref() {
        let path = issue_dir.join("history.jsonl");
        let prior_id = parse_id(cursor.last_history_id.as_deref());
        let mut highest = prior_id;
        let mut sorted: Vec<&Value> = histories.iter().collect();
        sorted.sort_by_key(|h| parse_id(h.get("id").and_then(|v| v.as_str())));
        for h in sorted {
            let id = parse_id(h.get("id").and_then(|v| v.as_str()));
            if id <= prior_id {
                continue;
            }
            bytes_written += append_jsonl(&path, h)?;
            if id > highest {
                highest = id;
            }
        }
        if highest != prior_id {
            cursor.last_history_id = highest.map(|n| n.to_string());
        }
    }

    Ok(IssueWriteOutcome { bytes_written, cursor })
}

fn parse_id(s: Option<&str>) -> Option<u64> {
    s.and_then(|s| s.parse::<u64>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_round_trips_through_value() {
        let c = CursorState {
            latest_updated_iso: Some("2026-05-08T09:00:00.000+0000".into()),
            issues: {
                let mut m = BTreeMap::new();
                m.insert(
                    "ENG-1".into(),
                    PerIssueCursor {
                        last_comment_id: Some("100".into()),
                        last_history_id: Some("200".into()),
                    },
                );
                m
            },
        };
        let v = c.clone().into_value();
        let back = CursorState::from_value(&v);
        assert_eq!(back.latest_updated_iso, c.latest_updated_iso);
        assert_eq!(
            back.issues.get("ENG-1").unwrap().last_comment_id,
            Some("100".into())
        );
    }

    #[test]
    fn parse_id_handles_missing_and_numeric() {
        assert_eq!(parse_id(None), None);
        assert_eq!(parse_id(Some("not-a-number")), None);
        assert_eq!(parse_id(Some("42")), Some(42));
    }
}
