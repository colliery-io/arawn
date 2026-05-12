//! Atlassian projections — `jira_issues`, `jira_comments`,
//! `jira_history`, `confluence_pages`.
//!
//! Mirror layout (from `arawn-feeds::templates::jira`):
//! ```text
//! <feed_dir>/<project_key>/<issue_key>/
//!     ├── issue.json
//!     ├── comments.jsonl
//!     └── history.jsonl
//! ```
//!
//! Assignee-tracker is a flatter shape with `<feed_dir>/<issue_key>/issue.json`
//! (no project subdir, no comments / history). We handle both by
//! treating any subdir that contains `issue.json` as a jira issue.
//!
//! Confluence layout (from `arawn-feeds::templates::confluence`):
//! ```text
//! <feed_dir>/<page_id>/
//!     ├── page.json
//!     └── body.storage.xml
//! ```

use std::io::BufRead;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde_json::Value;
use tracing::warn;

use crate::error::ProjectionError;
use crate::types::{Projection, ProjectionRow};

pub const JIRA_ISSUES: &str = "jira_issues";
pub const JIRA_COMMENTS: &str = "jira_comments";
pub const JIRA_HISTORY: &str = "jira_history";
pub const CONFLUENCE_PAGES: &str = "confluence_pages";

#[derive(Debug, Clone)]
pub struct JiraIssueProjection {
    pub id: String,
    pub feed_id: String,
    pub source_id: String, // issue key, e.g. "ENG-123"
    pub source_ts: DateTime<Utc>,
    pub project_key: Option<String>,
    pub summary: String,
    pub status: Option<String>,
    pub assignee: Option<String>,
    pub reporter: Option<String>,
    pub priority: Option<String>,
    pub labels: Vec<String>,
    pub body_text: String, // description
}

impl Projection for JiraIssueProjection {
    fn feed_type(&self) -> &'static str {
        JIRA_ISSUES
    }
    fn row(&self) -> ProjectionRow {
        let metadata = serde_json::json!({
            "project_key": self.project_key,
            "summary": self.summary,
            "status": self.status,
            "assignee": self.assignee,
            "reporter": self.reporter,
            "priority": self.priority,
            "labels": self.labels,
        });
        let title = if self.summary.is_empty() {
            self.source_id.clone()
        } else {
            format!("{}: {}", self.source_id, self.summary)
        };
        let body_combined = if self.body_text.is_empty() {
            self.summary.clone()
        } else {
            format!("{}\n\n{}", self.summary, self.body_text)
        };
        ProjectionRow {
            id: self.id.clone(),
            feed_id: self.feed_id.clone(),
            source_id: self.source_id.clone(),
            source_ts: self.source_ts,
            title,
            body_text: body_combined,
            feed_type: JIRA_ISSUES.to_string(),
            metadata,
        }
    }
}

#[derive(Debug, Clone)]
pub struct JiraCommentProjection {
    pub id: String,
    pub feed_id: String,
    pub source_id: String, // comment id
    pub source_ts: DateTime<Utc>,
    pub issue_key: String,
    pub author: Option<String>,
    pub body_text: String,
}

impl Projection for JiraCommentProjection {
    fn feed_type(&self) -> &'static str {
        JIRA_COMMENTS
    }
    fn row(&self) -> ProjectionRow {
        let title = format!(
            "comment on {} by {}",
            self.issue_key,
            self.author.as_deref().unwrap_or("?")
        );
        let metadata = serde_json::json!({
            "issue_key": self.issue_key,
            "author": self.author,
        });
        ProjectionRow {
            id: self.id.clone(),
            feed_id: self.feed_id.clone(),
            source_id: self.source_id.clone(),
            source_ts: self.source_ts,
            title,
            body_text: self.body_text.clone(),
            feed_type: JIRA_COMMENTS.to_string(),
            metadata,
        }
    }
}

#[derive(Debug, Clone)]
pub struct JiraHistoryProjection {
    pub id: String,
    pub feed_id: String,
    pub source_id: String, // changelog event id
    pub source_ts: DateTime<Utc>,
    pub issue_key: String,
    pub field: String,
    pub from_value: String,
    pub to_value: String,
    pub author: Option<String>,
}

impl Projection for JiraHistoryProjection {
    fn feed_type(&self) -> &'static str {
        JIRA_HISTORY
    }
    fn row(&self) -> ProjectionRow {
        let title = format!(
            "{}: {} changed {} → {}",
            self.issue_key, self.field, self.from_value, self.to_value
        );
        let body_text = format!(
            "{} {} {} {} → {}",
            self.field,
            self.author.as_deref().unwrap_or("?"),
            self.from_value,
            "to",
            self.to_value
        );
        let metadata = serde_json::json!({
            "issue_key": self.issue_key,
            "field": self.field,
            "from": self.from_value,
            "to": self.to_value,
            "author": self.author,
        });
        ProjectionRow {
            id: self.id.clone(),
            feed_id: self.feed_id.clone(),
            source_id: self.source_id.clone(),
            source_ts: self.source_ts,
            title,
            body_text,
            feed_type: JIRA_HISTORY.to_string(),
            metadata,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfluencePageProjection {
    pub id: String,
    pub feed_id: String,
    pub source_id: String, // page id
    pub source_ts: DateTime<Utc>,
    pub space_key: Option<String>,
    pub parent_id: Option<String>,
    pub title: String,
    pub version: Option<i64>,
    pub author: Option<String>,
    pub body_text: String,
}

impl Projection for ConfluencePageProjection {
    fn feed_type(&self) -> &'static str {
        CONFLUENCE_PAGES
    }
    fn row(&self) -> ProjectionRow {
        let metadata = serde_json::json!({
            "space_key": self.space_key,
            "parent_id": self.parent_id,
            "version": self.version,
            "author": self.author,
        });
        ProjectionRow {
            id: self.id.clone(),
            feed_id: self.feed_id.clone(),
            source_id: self.source_id.clone(),
            source_ts: self.source_ts,
            title: self.title.clone(),
            body_text: self.body_text.clone(),
            feed_type: CONFLUENCE_PAGES.to_string(),
            metadata,
        }
    }
}

fn hash_id(prefix: &str, feed_id: &str, source: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h = DefaultHasher::new();
    feed_id.hash(&mut h);
    "::".hash(&mut h);
    source.hash(&mut h);
    format!("{prefix}-{:016x}", h.finish())
}

fn parse_dt(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

/// Walk a Jira feed dir. Handles both project-tracker layout
/// (`<project>/<issue>/{issue.json,comments.jsonl,history.jsonl}`) and
/// the flatter assignee-tracker layout (`<issue>/issue.json`).
pub fn walk_jira_feed_dir(
    feed_id: &str,
    feed_dir: &Path,
) -> Result<JiraWalkResult, ProjectionError> {
    let mut out = JiraWalkResult::default();
    visit_jira(feed_id, feed_dir, &mut out, 0)?;
    Ok(out)
}

#[derive(Default, Debug)]
pub struct JiraWalkResult {
    pub issues: Vec<JiraIssueProjection>,
    pub comments: Vec<JiraCommentProjection>,
    pub history: Vec<JiraHistoryProjection>,
}

fn visit_jira(
    feed_id: &str,
    dir: &Path,
    out: &mut JiraWalkResult,
    depth: usize,
) -> Result<(), ProjectionError> {
    if depth > 3 {
        return Ok(());
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(it) => it,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    let mut subdirs: Vec<PathBuf> = Vec::new();
    let mut has_issue_json = false;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let ft = entry.file_type()?;
        if ft.is_file() && entry.file_name() == "issue.json" {
            has_issue_json = true;
        }
        if ft.is_dir() {
            subdirs.push(path);
        }
    }
    if has_issue_json {
        let issue_path = dir.join("issue.json");
        if let Some(issue) = read_jira_issue(feed_id, &issue_path)? {
            let issue_key = issue.source_id.clone();
            out.issues.push(issue);
            let comments_path = dir.join("comments.jsonl");
            if comments_path.exists() {
                read_jira_comments(feed_id, &issue_key, &comments_path, &mut out.comments)?;
            }
            let history_path = dir.join("history.jsonl");
            if history_path.exists() {
                read_jira_history(feed_id, &issue_key, &history_path, &mut out.history)?;
            }
        }
    }
    for sub in subdirs {
        visit_jira(feed_id, &sub, out, depth + 1)?;
    }
    Ok(())
}

fn read_jira_issue(
    feed_id: &str,
    path: &Path,
) -> Result<Option<JiraIssueProjection>, ProjectionError> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let v: Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(e) => {
            warn!(path = %path.display(), error = %e, "jira issue: unparseable json");
            return Ok(None);
        }
    };
    // Tolerate both wrapped (`{"issue": {...}}`) and raw issue shapes.
    let issue = v.get("issue").unwrap_or(&v);
    let key = match issue.get("key").and_then(|v| v.as_str()) {
        Some(k) => k.to_string(),
        None => {
            warn!(path = %path.display(), "jira issue: missing 'key'");
            return Ok(None);
        }
    };
    let fields = issue.get("fields").cloned().unwrap_or(Value::Null);
    let summary = fields
        .get("summary")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let status = fields
        .get("status")
        .and_then(|v| v.get("name"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let assignee = fields
        .get("assignee")
        .and_then(|v| v.get("displayName").or_else(|| v.get("name")))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let reporter = fields
        .get("reporter")
        .and_then(|v| v.get("displayName").or_else(|| v.get("name")))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let priority = fields
        .get("priority")
        .and_then(|v| v.get("name"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let project_key = fields
        .get("project")
        .and_then(|v| v.get("key"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| key.split('-').next().map(|s| s.to_string()));
    let labels: Vec<String> = fields
        .get("labels")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let description = fields
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let updated_str = fields
        .get("updated")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let source_ts = if updated_str.is_empty() {
        Utc::now()
    } else {
        parse_dt(updated_str)
    };

    Ok(Some(JiraIssueProjection {
        id: hash_id("ji", feed_id, &key),
        feed_id: feed_id.to_string(),
        source_id: key,
        source_ts,
        project_key,
        summary,
        status,
        assignee,
        reporter,
        priority,
        labels,
        body_text: description,
    }))
}

fn read_jira_comments(
    feed_id: &str,
    issue_key: &str,
    path: &Path,
    out: &mut Vec<JiraCommentProjection>,
) -> Result<(), ProjectionError> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                warn!(path = %path.display(), error = %e, "skip jira comment");
                continue;
            }
        };
        let cid = match v.get("id").and_then(|x| x.as_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let body = v.get("body").and_then(|x| x.as_str()).unwrap_or_default();
        let author = v
            .get("author")
            .and_then(|x| x.get("displayName").or_else(|| x.get("name")))
            .and_then(|x| x.as_str())
            .map(String::from);
        let updated = v
            .get("updated")
            .or_else(|| v.get("created"))
            .and_then(|x| x.as_str())
            .map(parse_dt)
            .unwrap_or_else(Utc::now);
        out.push(JiraCommentProjection {
            id: hash_id("jc", feed_id, &format!("{issue_key}:{cid}")),
            feed_id: feed_id.to_string(),
            source_id: cid,
            source_ts: updated,
            issue_key: issue_key.to_string(),
            author,
            body_text: body.to_string(),
        });
    }
    Ok(())
}

fn read_jira_history(
    feed_id: &str,
    issue_key: &str,
    path: &Path,
    out: &mut Vec<JiraHistoryProjection>,
) -> Result<(), ProjectionError> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                warn!(path = %path.display(), error = %e, "skip jira history");
                continue;
            }
        };
        let event_id = match v.get("id").and_then(|x| x.as_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let author = v
            .get("author")
            .and_then(|x| x.get("displayName").or_else(|| x.get("name")))
            .and_then(|x| x.as_str())
            .map(String::from);
        let created = v
            .get("created")
            .and_then(|x| x.as_str())
            .map(parse_dt)
            .unwrap_or_else(Utc::now);
        // Each changelog has an `items` array; the feed flattens these.
        // Tolerate both shapes — flattened single item vs wrapper.
        let items = v
            .get("items")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_else(|| vec![v.clone()]);
        for (i, item) in items.iter().enumerate() {
            let field = item
                .get("field")
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string();
            let from_value = item
                .get("fromString")
                .or_else(|| item.get("from"))
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string();
            let to_value = item
                .get("toString")
                .or_else(|| item.get("to"))
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string();
            let item_source = format!("{event_id}#{i}");
            out.push(JiraHistoryProjection {
                id: hash_id("jh", feed_id, &item_source),
                feed_id: feed_id.to_string(),
                source_id: item_source,
                source_ts: created,
                issue_key: issue_key.to_string(),
                field,
                from_value,
                to_value,
                author: author.clone(),
            });
        }
    }
    Ok(())
}

/// Walk a Confluence space-archive dir.
pub fn walk_confluence_feed_dir(
    feed_id: &str,
    feed_dir: &Path,
) -> Result<Vec<ConfluencePageProjection>, ProjectionError> {
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
        let page_dir = entry.path();
        let page_json = page_dir.join("page.json");
        if !page_json.exists() {
            continue;
        }
        let bytes = std::fs::read(&page_json)?;
        let v: Value = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(e) => {
                warn!(path = %page_json.display(), error = %e, "confluence page: unparseable");
                continue;
            }
        };
        let page_id = entry.file_name().to_string_lossy().to_string();
        let title = v
            .get("title")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string();
        let space_key = v
            .get("spaceKey")
            .or_else(|| v.pointer("/space/key"))
            .and_then(|x| x.as_str())
            .map(String::from);
        let parent_id = v
            .get("parentId")
            .or_else(|| v.pointer("/parent/id"))
            .and_then(|x| x.as_str())
            .map(String::from);
        let version = v
            .pointer("/version/number")
            .and_then(|x| x.as_i64());
        let author = v
            .pointer("/version/by/displayName")
            .or_else(|| v.pointer("/version/by/name"))
            .and_then(|x| x.as_str())
            .map(String::from);
        let updated = v
            .get("lastModified")
            .or_else(|| v.pointer("/version/when"))
            .and_then(|x| x.as_str())
            .map(parse_dt)
            .unwrap_or_else(Utc::now);
        let body_path = page_dir.join("body.storage.xml");
        let body_text = match std::fs::read_to_string(&body_path) {
            Ok(s) => s,
            Err(e) => {
                warn!(path = %body_path.display(), error = %e, "confluence body unavailable");
                String::new()
            }
        };
        out.push(ConfluencePageProjection {
            id: hash_id("cp", feed_id, &page_id),
            feed_id: feed_id.to_string(),
            source_id: page_id,
            source_ts: updated,
            space_key,
            parent_id,
            title,
            version,
            author,
            body_text,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn jira_issue_from_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let issue_dir = tmp.path().join("ENG-100");
        std::fs::create_dir(&issue_dir).unwrap();
        std::fs::write(
            issue_dir.join("issue.json"),
            json!({
                "key": "ENG-100",
                "fields": {
                    "summary": "Migrate auth",
                    "status": { "name": "In Progress" },
                    "assignee": { "displayName": "Alice" },
                    "reporter": { "displayName": "Bob" },
                    "priority": { "name": "High" },
                    "labels": ["backend", "auth"],
                    "description": "Long description...",
                    "updated": "2026-05-01T10:00:00Z",
                    "project": { "key": "ENG" }
                }
            })
            .to_string(),
        )
        .unwrap();

        let result = walk_jira_feed_dir("jt-feed", tmp.path()).unwrap();
        assert_eq!(result.issues.len(), 1);
        let i = &result.issues[0];
        assert_eq!(i.source_id, "ENG-100");
        assert_eq!(i.summary, "Migrate auth");
        assert_eq!(i.status.as_deref(), Some("In Progress"));
        assert_eq!(i.project_key.as_deref(), Some("ENG"));
        assert_eq!(i.labels, vec!["backend", "auth"]);
    }

    #[test]
    fn jira_comments_and_history() {
        let tmp = tempfile::tempdir().unwrap();
        let issue_dir = tmp.path().join("ENG/ENG-200");
        std::fs::create_dir_all(&issue_dir).unwrap();
        std::fs::write(
            issue_dir.join("issue.json"),
            json!({
                "key": "ENG-200",
                "fields": { "summary": "x" }
            })
            .to_string(),
        )
        .unwrap();
        std::fs::write(
            issue_dir.join("comments.jsonl"),
            json!({"id":"c1","body":"hi","author":{"displayName":"Alice"},"created":"2026-05-01T00:00:00Z"})
                .to_string()
                + "\n"
                + &json!({"id":"c2","body":"hello","author":{"displayName":"Bob"},"created":"2026-05-02T00:00:00Z"}).to_string(),
        )
        .unwrap();
        std::fs::write(
            issue_dir.join("history.jsonl"),
            json!({
                "id":"h1",
                "author":{"displayName":"Alice"},
                "created":"2026-05-01T00:00:00Z",
                "items":[{"field":"status","fromString":"Todo","toString":"In Progress"}]
            })
            .to_string(),
        )
        .unwrap();

        let result = walk_jira_feed_dir("jt", tmp.path()).unwrap();
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.comments.len(), 2);
        assert_eq!(result.history.len(), 1);
        assert_eq!(result.history[0].field, "status");
        assert_eq!(result.history[0].to_value, "In Progress");
    }

    #[test]
    fn confluence_page_from_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let page_dir = tmp.path().join("12345");
        std::fs::create_dir(&page_dir).unwrap();
        std::fs::write(
            page_dir.join("page.json"),
            json!({
                "title": "Onboarding",
                "space": { "key": "ENG" },
                "version": { "number": 5, "by": { "displayName": "Alice" }, "when": "2026-05-10T12:00:00Z" }
            }).to_string()
        ).unwrap();
        std::fs::write(
            page_dir.join("body.storage.xml"),
            "<p>welcome to engineering</p>",
        )
        .unwrap();

        let out = walk_confluence_feed_dir("ca-feed", tmp.path()).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].title, "Onboarding");
        assert_eq!(out[0].space_key.as_deref(), Some("ENG"));
        assert_eq!(out[0].version, Some(5));
        assert!(out[0].body_text.contains("welcome"));
    }
}
