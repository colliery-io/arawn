//! Linear projection — `linear_issues`.
//!
//! Mirror layout (from `arawn-feeds::templates::linear::my_issues`):
//! ```text
//! <feed_dir>/<issue id>/issue.json
//! ```
//! One row per issue, keyed by the stable issue id. The snapshot is
//! rewritten whenever the issue changes, so the dispatcher upserts every
//! row on each pass (state, priority and due date drive the morning
//! brief and must not go stale).

use std::path::Path;

use chrono::{DateTime, Utc};
use serde_json::Value;
use tracing::warn;

use crate::error::ProjectionError;
use crate::types::{Projection, ProjectionRow};

pub const FEED_TYPE: &str = "linear_issues";

#[derive(Debug, Clone)]
pub struct LinearIssueProjection {
    pub id: String,
    pub feed_id: String,
    pub source_id: String,        // issue id
    pub source_ts: DateTime<Utc>, // updatedAt
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub url: Option<String>,
    pub state: Option<String>,
    /// `triage | backlog | unstarted | started | completed | canceled`.
    pub state_type: Option<String>,
    /// 0 none, 1 urgent, 2 high, 3 medium, 4 low.
    pub priority: u8,
    pub priority_label: Option<String>,
    /// `YYYY-MM-DD`.
    pub due_date: Option<String>,
    pub team_key: Option<String>,
    pub labels: Vec<String>,
    pub assignee: Option<String>,
    pub creator: Option<String>,
    pub assigned_to_me: bool,
    /// `author: body`, newest last (the feed keeps the last ten).
    pub comments: Vec<String>,
}

impl Projection for LinearIssueProjection {
    fn feed_type(&self) -> &'static str {
        FEED_TYPE
    }

    fn row(&self) -> ProjectionRow {
        let mut body_text = self.title.clone();
        if !self.description.is_empty() {
            body_text.push_str("\n\n");
            body_text.push_str(&self.description);
        }
        for c in &self.comments {
            body_text.push_str("\n\n");
            body_text.push_str(c);
        }
        let metadata = serde_json::json!({
            "identifier": self.identifier,
            "url": self.url,
            "state": self.state,
            "state_type": self.state_type,
            "priority": self.priority,
            "priority_label": self.priority_label,
            "due_date": self.due_date,
            "team_key": self.team_key,
            "labels": self.labels,
            "assignee": self.assignee,
            "creator": self.creator,
            "assigned_to_me": self.assigned_to_me,
        });
        ProjectionRow {
            id: self.id.clone(),
            feed_id: self.feed_id.clone(),
            source_id: self.source_id.clone(),
            source_ts: self.source_ts,
            title: format!("{}: {}", self.identifier, self.title),
            body_text,
            feed_type: FEED_TYPE.to_string(),
            metadata,
        }
    }
}

fn hash_id(feed_id: &str, source: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h = DefaultHasher::new();
    feed_id.hash(&mut h);
    "::".hash(&mut h);
    source.hash(&mut h);
    format!("li-{:016x}", h.finish())
}

fn str_at<'a>(v: &'a Value, path: &str) -> Option<&'a str> {
    v.pointer(path).and_then(Value::as_str)
}

/// Parse one `issue.json` snapshot. `None` (with a warning) when the
/// required `id` / `identifier` are missing.
pub fn from_snapshot(feed_id: &str, v: &Value) -> Option<LinearIssueProjection> {
    let (Some(id), Some(identifier)) = (str_at(v, "/id"), str_at(v, "/identifier")) else {
        warn!("linear issue: snapshot without id/identifier");
        return None;
    };
    let source_ts = str_at(v, "/updated_at")
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let comments = v
        .get("comments")
        .and_then(Value::as_array)
        .map(|cs| {
            cs.iter()
                .filter_map(|c| {
                    let body = str_at(c, "/body")?;
                    Some(format!("{}: {body}", str_at(c, "/author").unwrap_or("?")))
                })
                .collect()
        })
        .unwrap_or_default();
    Some(LinearIssueProjection {
        id: hash_id(feed_id, id),
        feed_id: feed_id.to_string(),
        source_id: id.to_string(),
        source_ts,
        identifier: identifier.to_string(),
        title: str_at(v, "/title").unwrap_or_default().to_string(),
        description: str_at(v, "/description").unwrap_or_default().to_string(),
        url: str_at(v, "/url").map(String::from),
        state: str_at(v, "/state/name").map(String::from),
        state_type: str_at(v, "/state/type").map(String::from),
        priority: v
            .get("priority")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(4) as u8,
        priority_label: str_at(v, "/priority_label").map(String::from),
        due_date: str_at(v, "/due_date").map(String::from),
        team_key: str_at(v, "/team/key").map(String::from),
        labels: v
            .get("labels")
            .and_then(Value::as_array)
            .map(|ls| {
                ls.iter()
                    .filter_map(|l| l.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        assignee: str_at(v, "/assignee/name").map(String::from),
        creator: str_at(v, "/creator/name").map(String::from),
        assigned_to_me: v
            .get("assigned_to_me")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        comments,
    })
}

/// Walk `<feed_dir>/<issue id>/issue.json`.
pub fn walk_feed_dir(
    feed_id: &str,
    feed_dir: &Path,
) -> Result<Vec<LinearIssueProjection>, ProjectionError> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(feed_dir) {
        Ok(it) => it,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e.into()),
    };
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let path = entry.path().join("issue.json");
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e.into()),
        };
        match serde_json::from_slice::<Value>(&bytes) {
            Ok(v) => out.extend(from_snapshot(feed_id, &v)),
            Err(e) => warn!(path = %path.display(), error = %e, "skip unparseable linear issue"),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::ProjectionStore;
    use serde_json::json;

    fn snapshot(state_type: &str) -> Value {
        json!({
            "id": "uuid-7", "identifier": "ENG-7", "title": "Flaky checkout test",
            "description": "Fails on the payment step about one run in five.",
            "url": "https://linear.app/acme/issue/ENG-7",
            "state": {"name": "In Progress", "type": state_type},
            "priority": 1, "priority_label": "Urgent", "due_date": "2026-10-09",
            "team": {"key": "ENG", "name": "Engineering"},
            "labels": ["bug", "ci"],
            "assignee": {"id": "me", "name": "Ada", "email": null},
            "creator": {"id": "u2", "name": "Bea", "email": null},
            "assigned_to_me": true,
            "created_at": "2026-10-01T00:00:00.000Z",
            "updated_at": "2026-10-06T08:30:00.000Z",
            "comments": [{"id": "c1", "author": "Bea", "body": "Seen on staging too",
                          "created_at": "2026-10-02T00:00:00.000Z"}]
        })
    }

    #[test]
    fn snapshot_walks_into_a_row_found_by_fts() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("uuid-7");
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("issue.json"), snapshot("started").to_string()).unwrap();
        std::fs::write(tmp.path().join("meta.json"), "{}").unwrap();

        let rows = walk_feed_dir("linear-mine", tmp.path()).unwrap();
        assert_eq!(rows.len(), 1);
        let p = &rows[0];
        assert_eq!(p.source_id, "uuid-7");
        assert_eq!(p.priority, 1);
        assert!(p.assigned_to_me);
        let row = p.row();
        assert_eq!(row.title, "ENG-7: Flaky checkout test");
        assert_eq!(row.metadata["state_type"], "started");
        assert_eq!(row.metadata["due_date"], "2026-10-09");
        assert!(row.body_text.contains("Bea: Seen on staging too"));

        let store = ProjectionStore::in_memory().unwrap();
        store.write_batch(&rows).unwrap();
        // Description and comment text are both searchable.
        assert_eq!(
            store.fts_search(FEED_TYPE, "payment step", 5).unwrap(),
            std::slice::from_ref(&p.id)
        );
        assert_eq!(
            store.fts_search(FEED_TYPE, "staging", 5).unwrap(),
            std::slice::from_ref(&p.id)
        );
    }

    #[test]
    fn dispatch_upserts_so_a_closed_issue_stops_looking_open() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("uuid-7");
        std::fs::create_dir(&dir).unwrap();
        let store = ProjectionStore::in_memory().unwrap();
        let state_type = |store: &ProjectionStore| {
            let id = store
                .fts_search(FEED_TYPE, "checkout", 1)
                .unwrap()
                .remove(0);
            store.get_row(FEED_TYPE, &id).unwrap().unwrap().metadata["state_type"].clone()
        };

        std::fs::write(dir.join("issue.json"), snapshot("started").to_string()).unwrap();
        crate::project_feed_dir(&store, "linear/my-issues", "linear-mine", tmp.path()).unwrap();
        assert_eq!(state_type(&store), "started");

        std::fs::write(dir.join("issue.json"), snapshot("completed").to_string()).unwrap();
        crate::project_feed_dir(&store, "linear/my-issues", "linear-mine", tmp.path()).unwrap();
        assert_eq!(state_type(&store), "completed");
        assert_eq!(store.count(FEED_TYPE).unwrap(), 1);
    }

    #[test]
    fn snapshot_without_identity_is_skipped() {
        assert!(from_snapshot("f", &json!({"title": "x"})).is_none());
    }
}
