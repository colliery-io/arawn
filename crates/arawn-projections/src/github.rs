//! GitHub projections (I-0045 T-0318).
//!
//! Three feed types share this module:
//!
//! - [`NOTIFICATIONS_FEED_TYPE`] — the user's `/notifications` inbox.
//!   Metadata-heavy (subject, reason, thread_id); body_text is the
//!   subject title (notification API doesn't expose the linked
//!   issue/PR body inline).
//! - [`ISSUES_AND_PRS_FEED_TYPE`] — open + recently-closed issues
//!   and PRs the user authored or is assigned to. body_text is the
//!   title + body excerpt capped at ~1 KB.
//! - [`REVIEW_QUEUE_FEED_TYPE`] — PRs where the user is a
//!   requested reviewer. body_text is the title; the `draft` flag
//!   lives in metadata so the morning brief can deprioritise drafts.
//!
//! The actual REST-API parsers (`from_notification_json`,
//! `from_issue_or_pr_json`, `from_review_request_json`) live here so
//! the feed templates in T-0319 / T-0320 / T-0321 are thin wrappers.

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::types::{Projection, ProjectionRow};

pub const NOTIFICATIONS_FEED_TYPE: &str = "github_notifications";
pub const ISSUES_AND_PRS_FEED_TYPE: &str = "github_issues_and_prs";
pub const REVIEW_QUEUE_FEED_TYPE: &str = "github_review_queue";

/// Cap on body excerpts. GitHub issue/PR bodies can be megabytes
/// (auto-generated changelogs, etc.); we keep only the leading slice.
pub const BODY_EXCERPT_MAX: usize = 1024;

// =============================================================================
// github_notifications
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct GithubNotificationProjection {
    pub id: String,
    pub feed_id: String,
    /// API's `id` field (string in GitHub's REST API even though it's
    /// numeric; we keep the string form).
    pub source_id: String,
    pub source_ts: DateTime<Utc>,
    pub owner: String,
    pub repo: String,
    pub thread_id: String,
    pub subject_title: String,
    /// API-side URL — `https://api.github.com/repos/<owner>/<repo>/
    /// issues/<n>` or similar. Web URL is derived at render time.
    pub subject_url: Option<String>,
    /// `assign` | `author` | `comment` | `mention` | `review_requested` | etc.
    pub reason: String,
    /// `Issue` | `PullRequest` | `Discussion` | `Commit` | `Release`.
    pub kind: String,
    pub unread: bool,
    pub updated_at: DateTime<Utc>,
    pub last_read_at: Option<DateTime<Utc>>,
}

impl Projection for GithubNotificationProjection {
    fn feed_type(&self) -> &'static str {
        NOTIFICATIONS_FEED_TYPE
    }

    fn row(&self) -> ProjectionRow {
        let metadata = serde_json::json!({
            "owner": self.owner,
            "repo": self.repo,
            "thread_id": self.thread_id,
            "subject_url": self.subject_url,
            "reason": self.reason,
            "kind": self.kind,
            "unread": self.unread,
            "updated_at": self.updated_at.to_rfc3339(),
            "last_read_at": self.last_read_at.map(|d| d.to_rfc3339()),
        });
        ProjectionRow {
            id: self.id.clone(),
            feed_id: self.feed_id.clone(),
            source_id: self.source_id.clone(),
            source_ts: self.source_ts,
            title: if self.subject_title.is_empty() {
                format!("{}/{} notification", self.owner, self.repo)
            } else {
                self.subject_title.clone()
            },
            body_text: self.subject_title.clone(),
            feed_type: NOTIFICATIONS_FEED_TYPE.to_string(),
            metadata,
        }
    }
}

/// Parse a GitHub `/notifications` API response item. Returns `None`
/// when the JSON shape doesn't look like a notification — defensive
/// because GitHub occasionally drops new event kinds in here.
pub fn from_notification_json(feed_id: &str, v: &Value) -> Option<GithubNotificationProjection> {
    let source_id = v.get("id")?.as_str()?.to_string();
    let subject = v.get("subject")?;
    let subject_title = subject
        .get("title")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    let subject_url = subject
        .get("url")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string());
    let kind = subject
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    let reason = v
        .get("reason")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    let unread = v.get("unread").and_then(|t| t.as_bool()).unwrap_or(false);
    let updated_at = parse_dt(v.get("updated_at"))?;
    let last_read_at = parse_dt_opt(v.get("last_read_at"));
    let repo = v.get("repository")?;
    let owner = repo
        .get("owner")
        .and_then(|o| o.get("login"))
        .and_then(|n| n.as_str())?
        .to_string();
    let repo_name = repo.get("name").and_then(|n| n.as_str())?.to_string();
    let thread_id = source_id.clone();
    Some(GithubNotificationProjection {
        id: format!("{feed_id}:{source_id}"),
        feed_id: feed_id.to_string(),
        source_id,
        source_ts: updated_at,
        owner,
        repo: repo_name,
        thread_id,
        subject_title,
        subject_url,
        reason,
        kind,
        unread,
        updated_at,
        last_read_at,
    })
}

// =============================================================================
// github_issues_and_prs
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct GithubIssueOrPrProjection {
    pub id: String,
    pub feed_id: String,
    pub source_id: String,
    pub source_ts: DateTime<Utc>,
    pub owner: String,
    pub repo: String,
    pub number: i64,
    /// `issue` or `pr`.
    pub kind: String,
    pub title: String,
    pub url: String,
    /// `open` | `closed` (or `merged` for PRs that have been merged).
    pub state: String,
    pub author: String,
    pub assignees: Vec<String>,
    pub labels: Vec<String>,
    /// Capped at [`BODY_EXCERPT_MAX`].
    pub body_excerpt: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub merged_at: Option<DateTime<Utc>>,
}

impl Projection for GithubIssueOrPrProjection {
    fn feed_type(&self) -> &'static str {
        ISSUES_AND_PRS_FEED_TYPE
    }

    fn row(&self) -> ProjectionRow {
        let metadata = serde_json::json!({
            "owner": self.owner,
            "repo": self.repo,
            "number": self.number,
            "kind": self.kind,
            "url": self.url,
            "state": self.state,
            "author": self.author,
            "assignees": self.assignees,
            "labels": self.labels,
            "created_at": self.created_at.to_rfc3339(),
            "updated_at": self.updated_at.to_rfc3339(),
            "closed_at": self.closed_at.map(|d| d.to_rfc3339()),
            "merged_at": self.merged_at.map(|d| d.to_rfc3339()),
        });
        let body_text = if self.body_excerpt.is_empty() {
            self.title.clone()
        } else {
            format!("{}\n\n{}", self.title, self.body_excerpt)
        };
        ProjectionRow {
            id: self.id.clone(),
            feed_id: self.feed_id.clone(),
            source_id: self.source_id.clone(),
            source_ts: self.source_ts,
            title: self.title.clone(),
            body_text,
            feed_type: ISSUES_AND_PRS_FEED_TYPE.to_string(),
            metadata,
        }
    }
}

/// Parse one row from `/search/issues` (issues or PRs — the search
/// endpoint returns the same shape for both, discriminated by the
/// presence of `pull_request` on the row).
pub fn from_issue_or_pr_json(feed_id: &str, v: &Value) -> Option<GithubIssueOrPrProjection> {
    let url = v.get("html_url")?.as_str()?.to_string();
    let (owner, repo, number) = parse_html_url_owner_repo_number(&url)?;
    let title = v.get("title")?.as_str()?.to_string();
    let state_raw = v
        .get("state")
        .and_then(|s| s.as_str())
        .unwrap_or("open")
        .to_string();
    // PRs distinguished by presence of `pull_request` field.
    let pull_request = v.get("pull_request");
    let kind = if pull_request.is_some() { "pr" } else { "issue" }.to_string();
    let merged_at = pull_request
        .and_then(|p| p.get("merged_at"))
        .and_then(|t| t.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc));
    let state = if merged_at.is_some() {
        "merged".to_string()
    } else {
        state_raw
    };
    let author = v
        .get("user")
        .and_then(|u| u.get("login"))
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_string();
    let assignees: Vec<String> = v
        .get("assignees")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.get("login").and_then(|l| l.as_str()))
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();
    let labels: Vec<String> = v
        .get("labels")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.get("name").and_then(|n| n.as_str()))
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();
    let raw_body = v.get("body").and_then(|b| b.as_str()).unwrap_or("");
    let body_excerpt = truncate_chars(raw_body, BODY_EXCERPT_MAX);
    let created_at = parse_dt(v.get("created_at"))?;
    let updated_at = parse_dt(v.get("updated_at"))?;
    let closed_at = parse_dt_opt(v.get("closed_at"));
    let source_id = format!("{owner}/{repo}#{number}");
    Some(GithubIssueOrPrProjection {
        id: format!("{feed_id}:{source_id}"),
        feed_id: feed_id.to_string(),
        source_id,
        source_ts: updated_at,
        owner,
        repo,
        number,
        kind,
        title,
        url,
        state,
        author,
        assignees,
        labels,
        body_excerpt,
        created_at,
        updated_at,
        closed_at,
        merged_at,
    })
}

// =============================================================================
// github_review_queue
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct GithubReviewRequestProjection {
    pub id: String,
    pub feed_id: String,
    pub source_id: String,
    pub source_ts: DateTime<Utc>,
    pub owner: String,
    pub repo: String,
    pub pr_number: i64,
    pub title: String,
    pub url: String,
    pub author: String,
    pub requested_at: DateTime<Utc>,
    pub draft: bool,
}

impl Projection for GithubReviewRequestProjection {
    fn feed_type(&self) -> &'static str {
        REVIEW_QUEUE_FEED_TYPE
    }

    fn row(&self) -> ProjectionRow {
        let metadata = serde_json::json!({
            "owner": self.owner,
            "repo": self.repo,
            "pr_number": self.pr_number,
            "url": self.url,
            "author": self.author,
            "requested_at": self.requested_at.to_rfc3339(),
            "draft": self.draft,
        });
        ProjectionRow {
            id: self.id.clone(),
            feed_id: self.feed_id.clone(),
            source_id: self.source_id.clone(),
            source_ts: self.source_ts,
            title: self.title.clone(),
            body_text: self.title.clone(),
            feed_type: REVIEW_QUEUE_FEED_TYPE.to_string(),
            metadata,
        }
    }
}

/// Parse one row from `/search/issues?q=is:pr is:open
/// review-requested:@me`. The search endpoint doesn't return
/// `requested_reviewers`, so we treat the row's `updated_at` as the
/// best proxy for "when did this PR first need my eyes". Refining
/// this requires a separate per-PR fetch which T-0321 may add.
pub fn from_review_request_json(feed_id: &str, v: &Value) -> Option<GithubReviewRequestProjection> {
    let url = v.get("html_url")?.as_str()?.to_string();
    let (owner, repo, pr_number) = parse_html_url_owner_repo_number(&url)?;
    let title = v.get("title")?.as_str()?.to_string();
    let author = v
        .get("user")
        .and_then(|u| u.get("login"))
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_string();
    let requested_at = parse_dt(v.get("updated_at"))?;
    let draft = v.get("draft").and_then(|d| d.as_bool()).unwrap_or(false);
    let source_id = format!("{owner}/{repo}#{pr_number}");
    Some(GithubReviewRequestProjection {
        id: format!("{feed_id}:{source_id}"),
        feed_id: feed_id.to_string(),
        source_id,
        source_ts: requested_at,
        owner,
        repo,
        pr_number,
        title,
        url,
        author,
        requested_at,
        draft,
    })
}

// =============================================================================
// Shared helpers
// =============================================================================

fn parse_dt(v: Option<&Value>) -> Option<DateTime<Utc>> {
    v.and_then(|x| x.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc))
}

fn parse_dt_opt(v: Option<&Value>) -> Option<DateTime<Utc>> {
    match v {
        None => None,
        Some(Value::Null) => None,
        Some(other) => parse_dt(Some(other)),
    }
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i >= max {
            break;
        }
        out.push(c);
    }
    out
}

/// Walk a `github/notifications` feed dir. Each `.json` under
/// `<feed_dir>/notifications/` is one raw notification payload.
pub fn walk_notifications_dir(
    feed_id: &str,
    feed_dir: &std::path::Path,
) -> Result<Vec<GithubNotificationProjection>, crate::error::ProjectionError> {
    let dir = feed_dir.join("notifications");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| {
        crate::error::ProjectionError::Storage(format!("read_dir {}: {e}", dir.display()))
    })?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let body = std::fs::read_to_string(&path).map_err(|e| {
            crate::error::ProjectionError::Storage(format!("read {}: {e}", path.display()))
        })?;
        let v: Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(_) => continue, // partial/corrupt file — skip silently
        };
        if let Some(p) = from_notification_json(feed_id, &v) {
            out.push(p);
        }
    }
    Ok(out)
}

/// Walk a `github/issues-and-prs` feed dir. Each `.json` under
/// `<feed_dir>/issues_and_prs/` is one raw search-result row.
pub fn walk_issues_and_prs_dir(
    feed_id: &str,
    feed_dir: &std::path::Path,
) -> Result<Vec<GithubIssueOrPrProjection>, crate::error::ProjectionError> {
    walk_simple_dir("issues_and_prs", feed_dir, |v| {
        from_issue_or_pr_json(feed_id, v)
    })
}

/// Walk a `github/review-queue` feed dir.
pub fn walk_review_queue_dir(
    feed_id: &str,
    feed_dir: &std::path::Path,
) -> Result<Vec<GithubReviewRequestProjection>, crate::error::ProjectionError> {
    walk_simple_dir("review_queue", feed_dir, |v| {
        from_review_request_json(feed_id, v)
    })
}

fn walk_simple_dir<T>(
    subdir: &str,
    feed_dir: &std::path::Path,
    parse: impl Fn(&Value) -> Option<T>,
) -> Result<Vec<T>, crate::error::ProjectionError> {
    let dir = feed_dir.join(subdir);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| {
        crate::error::ProjectionError::Storage(format!("read_dir {}: {e}", dir.display()))
    })?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let body = std::fs::read_to_string(&path).map_err(|e| {
            crate::error::ProjectionError::Storage(format!("read {}: {e}", path.display()))
        })?;
        let v: Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(p) = parse(&v) {
            out.push(p);
        }
    }
    Ok(out)
}

/// Extract `(owner, repo, number)` from a GitHub web URL like
/// `https://github.com/openai/codex/issues/123`.
fn parse_html_url_owner_repo_number(url: &str) -> Option<(String, String, i64)> {
    let path = url.strip_prefix("https://github.com/")?;
    let mut parts = path.splitn(4, '/');
    let owner = parts.next()?.to_string();
    let repo = parts.next()?.to_string();
    let _kind = parts.next()?; // "issues" or "pull"
    let rest = parts.next()?;
    let number_str = rest.split(['/', '#', '?']).next().unwrap_or(rest);
    let number = number_str.parse::<i64>().ok()?;
    Some((owner, repo, number))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn assert_round_trips<P: Projection + Clone + std::fmt::Debug>(p: &P) {
        let row = p.row();
        assert!(!row.id.is_empty());
        assert!(!row.feed_id.is_empty());
        assert!(!row.title.is_empty());
        assert!(row.metadata.is_object());
    }

    #[test]
    fn notification_round_trip() {
        let v = json!({
            "id": "12345",
            "unread": true,
            "reason": "review_requested",
            "updated_at": "2026-05-18T10:00:00Z",
            "last_read_at": null,
            "subject": {
                "title": "Ship the GitHub integration",
                "url": "https://api.github.com/repos/arawn/arawn/pulls/42",
                "type": "PullRequest"
            },
            "repository": {
                "name": "arawn",
                "owner": { "login": "arawn-dev" }
            }
        });
        let p = from_notification_json("feed-1", &v).expect("parsed");
        assert_eq!(p.source_id, "12345");
        assert_eq!(p.owner, "arawn-dev");
        assert_eq!(p.repo, "arawn");
        assert_eq!(p.reason, "review_requested");
        assert_eq!(p.kind, "PullRequest");
        assert!(p.unread);
        assert!(p.last_read_at.is_none());
        assert_round_trips(&p);
        // Metadata JSON shape.
        let row = p.row();
        assert_eq!(row.metadata["unread"], true);
        assert_eq!(row.metadata["reason"], "review_requested");
        assert_eq!(row.metadata["thread_id"], "12345");
        assert_eq!(row.feed_type, NOTIFICATIONS_FEED_TYPE);
    }

    #[test]
    fn issue_round_trip() {
        let v = json!({
            "html_url": "https://github.com/openai/codex/issues/42",
            "title": "Reproduce the panic",
            "state": "open",
            "user": { "login": "alice" },
            "assignees": [{"login":"bob"}, {"login":"carol"}],
            "labels": [{"name":"bug"}, {"name":"high"}],
            "body": "Steps to reproduce: ...",
            "created_at": "2026-05-10T09:00:00Z",
            "updated_at": "2026-05-18T09:00:00Z",
            "closed_at": null
        });
        let p = from_issue_or_pr_json("feed-1", &v).expect("parsed");
        assert_eq!(p.kind, "issue");
        assert_eq!(p.owner, "openai");
        assert_eq!(p.repo, "codex");
        assert_eq!(p.number, 42);
        assert_eq!(p.assignees, vec!["bob".to_string(), "carol".to_string()]);
        assert_eq!(p.labels, vec!["bug".to_string(), "high".to_string()]);
        assert_eq!(p.state, "open");
        assert!(p.merged_at.is_none());
        assert!(p.closed_at.is_none());
        assert_round_trips(&p);
    }

    #[test]
    fn pr_with_merged_at_reports_state_merged() {
        let v = json!({
            "html_url": "https://github.com/openai/codex/pull/7",
            "title": "Land the thing",
            "state": "closed",
            "user": { "login": "alice" },
            "pull_request": { "merged_at": "2026-05-17T18:00:00Z" },
            "body": "",
            "created_at": "2026-05-10T09:00:00Z",
            "updated_at": "2026-05-17T18:00:00Z"
        });
        let p = from_issue_or_pr_json("feed-1", &v).expect("parsed");
        assert_eq!(p.kind, "pr");
        assert_eq!(p.state, "merged");
        assert!(p.merged_at.is_some());
    }

    #[test]
    fn body_excerpt_truncated_to_max() {
        let body: String = std::iter::repeat('x').take(BODY_EXCERPT_MAX + 50).collect();
        let v = json!({
            "html_url": "https://github.com/o/r/issues/1",
            "title": "long body",
            "state": "open",
            "user": {"login":"a"},
            "body": body,
            "created_at": "2026-05-10T09:00:00Z",
            "updated_at": "2026-05-18T09:00:00Z"
        });
        let p = from_issue_or_pr_json("feed", &v).expect("parsed");
        assert_eq!(p.body_excerpt.chars().count(), BODY_EXCERPT_MAX);
    }

    #[test]
    fn review_request_round_trip() {
        let v = json!({
            "html_url": "https://github.com/openai/codex/pull/9",
            "title": "Add the docs",
            "user": {"login":"alice"},
            "updated_at": "2026-05-18T10:00:00Z",
            "draft": false
        });
        let p = from_review_request_json("feed-1", &v).expect("parsed");
        assert_eq!(p.owner, "openai");
        assert_eq!(p.repo, "codex");
        assert_eq!(p.pr_number, 9);
        assert!(!p.draft);
        assert_round_trips(&p);
    }

    #[test]
    fn missing_required_fields_returns_none() {
        // No subject → not a notification.
        let v = json!({"id":"1","updated_at":"2026-05-18T10:00:00Z","repository":{"name":"r","owner":{"login":"o"}}});
        assert!(from_notification_json("f", &v).is_none());
        // No html_url → not an issue/PR.
        let v = json!({"title":"x"});
        assert!(from_issue_or_pr_json("f", &v).is_none());
    }

    #[test]
    fn walks_notifications_dir_skipping_garbage() {
        let tmp = tempfile::tempdir().unwrap();
        let notif_dir = tmp.path().join("notifications");
        std::fs::create_dir(&notif_dir).unwrap();
        std::fs::write(
            notif_dir.join("1.json"),
            json!({
                "id":"1","unread":true,"reason":"review_requested",
                "updated_at":"2026-05-18T10:00:00Z","last_read_at":null,
                "subject":{"title":"a","url":"u","type":"Issue"},
                "repository":{"name":"r","owner":{"login":"o"}}
            })
            .to_string(),
        )
        .unwrap();
        // Garbage file — should be silently skipped.
        std::fs::write(notif_dir.join("bad.json"), "not json").unwrap();
        // Wrong extension — ignored.
        std::fs::write(notif_dir.join("ignored.txt"), "irrelevant").unwrap();
        let parsed = walk_notifications_dir("feed-1", tmp.path()).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].source_id, "1");
    }

    #[test]
    fn walks_returns_empty_when_dir_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let parsed = walk_notifications_dir("feed-1", tmp.path()).unwrap();
        assert!(parsed.is_empty());
    }

    #[test]
    fn url_parser_handles_issues_and_prs() {
        assert_eq!(
            parse_html_url_owner_repo_number("https://github.com/foo/bar/issues/123"),
            Some(("foo".into(), "bar".into(), 123))
        );
        assert_eq!(
            parse_html_url_owner_repo_number("https://github.com/foo/bar/pull/7"),
            Some(("foo".into(), "bar".into(), 7))
        );
        assert!(parse_html_url_owner_repo_number("https://example.com/x/y/z/1").is_none());
    }
}
