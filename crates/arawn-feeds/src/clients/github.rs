//! GitHub — what feeds need from GitHub, plus the production
//! adapter over `arawn-integrations::github`.
//!
//! Templates depend on the [`GithubFeedClient`] trait. Tests fake it
//! externally; production wires [`RealGithubClient`], which holds a
//! `GithubIntegration` and mints installation-access-tokens on demand
//! (see `arawn_integrations::github::GithubClient`).
//!
//! Surface:
//!
//! - `list_notifications` for `github/notifications` (T-0319).
//! - `search_issues` for `github/issues-and-prs` (T-0320) and
//!   `github/review-queue` (T-0321).
//! - Six repo/org-scoped calls for `github/repo-mirror` (T-0323+):
//!   `list_repo_commits`, `list_repo_issues`, `list_repo_prs`,
//!   `list_issue_comments`, `list_pr_review_comments`,
//!   `list_org_repos`.

use std::sync::Arc;

use arawn_integrations::github::GithubIntegration;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::error::FeedError;

/// Authenticated GitHub REST calls templates rely on.
#[async_trait]
pub trait GithubFeedClient: Send + Sync {
    /// List notifications on the user's `/notifications` inbox,
    /// updated at or after `since` (None = no time-floor). Returns
    /// the raw JSON page so templates preserve full fidelity on
    /// disk. `per_page` capped by GitHub at 50; we follow `Link:
    /// rel="next"` headers if `all=true`.
    async fn list_notifications(
        &self,
        since: Option<DateTime<Utc>>,
        per_page: u32,
        all: bool,
    ) -> Result<Vec<Value>, FeedError>;

    /// Hit `/search/issues` with the given query string. Returns
    /// the `items` array from each page concatenated. `per_page`
    /// capped by GitHub at 100; pagination via `Link: rel="next"`.
    /// Hard-capped at `max_pages` to keep a single tick under the
    /// secondary rate limit (30 search calls / minute).
    async fn search_issues(
        &self,
        query: &str,
        per_page: u32,
        max_pages: u32,
    ) -> Result<Vec<Value>, FeedError>;

    /// Repo commits, newest-first. `since` filters by commit
    /// committer-date (RFC3339); None = no time-floor (capped via
    /// `max_pages`).
    async fn list_repo_commits(
        &self,
        owner: &str,
        repo: &str,
        since: Option<DateTime<Utc>>,
        max_pages: u32,
    ) -> Result<Vec<Value>, FeedError>;

    /// Repo issues. GitHub's `/issues` endpoint returns PRs too; this
    /// method filters them out client-side via `pull_request` field
    /// absence so the caller doesn't have to think about it.
    /// `state` ∈ {open, closed, all}.
    async fn list_repo_issues(
        &self,
        owner: &str,
        repo: &str,
        state: &str,
        since: Option<DateTime<Utc>>,
        max_pages: u32,
    ) -> Result<Vec<Value>, FeedError>;

    /// Repo pull requests. `state` ∈ {open, closed, all}.
    async fn list_repo_prs(
        &self,
        owner: &str,
        repo: &str,
        state: &str,
        since: Option<DateTime<Utc>>,
        max_pages: u32,
    ) -> Result<Vec<Value>, FeedError>;

    /// All issue comments on a repo (issues + PRs share this endpoint),
    /// updated at or after `since`.
    async fn list_issue_comments(
        &self,
        owner: &str,
        repo: &str,
        since: Option<DateTime<Utc>>,
        max_pages: u32,
    ) -> Result<Vec<Value>, FeedError>;

    /// PR review comments (the inline-code variety), updated at or
    /// after `since`.
    async fn list_pr_review_comments(
        &self,
        owner: &str,
        repo: &str,
        since: Option<DateTime<Utc>>,
        max_pages: u32,
    ) -> Result<Vec<Value>, FeedError>;

    /// List all repositories in an organisation. Used by the org-
    /// expand-at-register step (T-0327) to fan a single `github:
    /// org:owner` binding into N per-repo feeds.
    async fn list_org_repos(&self, owner: &str, max_pages: u32) -> Result<Vec<Value>, FeedError>;
}

// ─── Production adapter ──────────────────────────────────────────────

pub struct RealGithubClient {
    integration: Arc<GithubIntegration>,
}

impl RealGithubClient {
    pub fn new(integration: Arc<GithubIntegration>) -> Self {
        Self { integration }
    }
}

#[async_trait]
impl GithubFeedClient for RealGithubClient {
    async fn list_notifications(
        &self,
        since: Option<DateTime<Utc>>,
        per_page: u32,
        all: bool,
    ) -> Result<Vec<Value>, FeedError> {
        let client = self
            .integration
            .client()
            .map_err(|e| FeedError::Auth(format!("github: {e}")))?;
        let mut path = format!("/notifications?per_page={per_page}");
        if let Some(s) = since {
            // GitHub's `since` is RFC3339 (ISO 8601).
            path.push_str(&format!("&since={}", s.to_rfc3339()));
        }
        if all {
            path.push_str("&all=true");
        }

        let mut out: Vec<Value> = Vec::new();
        let mut next_path: Option<String> = Some(path);

        while let Some(p) = next_path.take() {
            let resp = client
                .get(&p)
                .await
                .map_err(|e| FeedError::Provider(format!("github get notifications: {e}")))?;
            let status = resp.status();
            // GitHub returns 304 Not Modified on If-Modified-Since;
            // we don't set that header here (since= is enough). 200
            // is the happy path.
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(FeedError::Provider(format!(
                    "github list_notifications: {status} body={body}"
                )));
            }
            // Save the Link header BEFORE consuming the body.
            let link_header = resp
                .headers()
                .get(reqwest::header::LINK)
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());
            let page: Vec<Value> = resp
                .json()
                .await
                .map_err(|e| FeedError::Provider(format!("github notifications parse: {e}")))?;
            out.extend(page);
            if !all {
                break;
            }
            // Follow Link: <url>; rel="next".
            if let Some(link) = link_header {
                next_path = parse_link_next_path(&link);
            }
        }
        Ok(out)
    }

    async fn search_issues(
        &self,
        query: &str,
        per_page: u32,
        max_pages: u32,
    ) -> Result<Vec<Value>, FeedError> {
        let client = self
            .integration
            .client()
            .map_err(|e| FeedError::Auth(format!("github: {e}")))?;
        let encoded = urlencoding::encode(query);
        let path =
            format!("/search/issues?q={encoded}&per_page={per_page}&sort=updated&order=desc",);
        let mut out: Vec<Value> = Vec::new();
        let mut next_path: Option<String> = Some(path);
        let mut pages_fetched: u32 = 0;
        while let Some(p) = next_path.take() {
            if pages_fetched >= max_pages {
                break;
            }
            let resp = client
                .get(&p)
                .await
                .map_err(|e| FeedError::Provider(format!("github search: {e}")))?;
            let status = resp.status();
            let link_header = resp
                .headers()
                .get(reqwest::header::LINK)
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(FeedError::Provider(format!(
                    "github search_issues: {status} body={body}"
                )));
            }
            let page: Value = resp
                .json()
                .await
                .map_err(|e| FeedError::Provider(format!("github search parse: {e}")))?;
            if let Some(items) = page.get("items").and_then(|i| i.as_array()) {
                out.extend(items.iter().cloned());
            }
            pages_fetched += 1;
            if let Some(link) = link_header {
                next_path = parse_link_next_path(&link);
            }
        }
        Ok(out)
    }

    async fn list_repo_commits(
        &self,
        owner: &str,
        repo: &str,
        since: Option<DateTime<Utc>>,
        max_pages: u32,
    ) -> Result<Vec<Value>, FeedError> {
        let mut path = format!("/repos/{owner}/{repo}/commits?per_page=100");
        if let Some(s) = since {
            path.push_str(&format!("&since={}", s.to_rfc3339()));
        }
        self.paginate_array(&path, max_pages, "list_repo_commits")
            .await
    }

    async fn list_repo_issues(
        &self,
        owner: &str,
        repo: &str,
        state: &str,
        since: Option<DateTime<Utc>>,
        max_pages: u32,
    ) -> Result<Vec<Value>, FeedError> {
        let state_enc = urlencoding::encode(state);
        let mut path = format!(
            "/repos/{owner}/{repo}/issues?state={state_enc}&per_page=100&sort=updated&direction=desc",
        );
        if let Some(s) = since {
            path.push_str(&format!("&since={}", s.to_rfc3339()));
        }
        let mixed = self
            .paginate_array(&path, max_pages, "list_repo_issues")
            .await?;
        Ok(strip_pr_rows(mixed))
    }

    async fn list_repo_prs(
        &self,
        owner: &str,
        repo: &str,
        state: &str,
        since: Option<DateTime<Utc>>,
        max_pages: u32,
    ) -> Result<Vec<Value>, FeedError> {
        let state_enc = urlencoding::encode(state);
        let path = format!(
            "/repos/{owner}/{repo}/pulls?state={state_enc}&per_page=100&sort=updated&direction=desc",
        );
        // GitHub's `/pulls` endpoint doesn't accept `since=` — it
        // sorts by updated desc instead. Filter client-side after
        // fetching, capped by `max_pages` to bound the worst case.
        let all = self
            .paginate_array(&path, max_pages, "list_repo_prs")
            .await?;
        Ok(filter_by_updated_at(all, since))
    }

    async fn list_issue_comments(
        &self,
        owner: &str,
        repo: &str,
        since: Option<DateTime<Utc>>,
        max_pages: u32,
    ) -> Result<Vec<Value>, FeedError> {
        let mut path = format!(
            "/repos/{owner}/{repo}/issues/comments?per_page=100&sort=updated&direction=desc",
        );
        if let Some(s) = since {
            path.push_str(&format!("&since={}", s.to_rfc3339()));
        }
        self.paginate_array(&path, max_pages, "list_issue_comments")
            .await
    }

    async fn list_pr_review_comments(
        &self,
        owner: &str,
        repo: &str,
        since: Option<DateTime<Utc>>,
        max_pages: u32,
    ) -> Result<Vec<Value>, FeedError> {
        let mut path = format!(
            "/repos/{owner}/{repo}/pulls/comments?per_page=100&sort=updated&direction=desc",
        );
        if let Some(s) = since {
            path.push_str(&format!("&since={}", s.to_rfc3339()));
        }
        self.paginate_array(&path, max_pages, "list_pr_review_comments")
            .await
    }

    async fn list_org_repos(&self, owner: &str, max_pages: u32) -> Result<Vec<Value>, FeedError> {
        let path = format!("/orgs/{owner}/repos?per_page=100&type=all&sort=updated");
        self.paginate_array(&path, max_pages, "list_org_repos")
            .await
    }
}

impl RealGithubClient {
    /// Shared paginator for endpoints that return a top-level JSON
    /// array. Follows `Link: rel="next"` up to `max_pages`. Surfaces
    /// non-2xx with response body for visibility.
    async fn paginate_array(
        &self,
        initial_path: &str,
        max_pages: u32,
        op_name: &str,
    ) -> Result<Vec<Value>, FeedError> {
        let client = self
            .integration
            .client()
            .map_err(|e| FeedError::Auth(format!("github: {e}")))?;
        let mut out: Vec<Value> = Vec::new();
        let mut next_path: Option<String> = Some(initial_path.to_string());
        let mut pages_fetched: u32 = 0;
        while let Some(p) = next_path.take() {
            if pages_fetched >= max_pages {
                break;
            }
            let resp = client
                .get(&p)
                .await
                .map_err(|e| FeedError::Provider(format!("github {op_name}: {e}")))?;
            let status = resp.status();
            let link_header = resp
                .headers()
                .get(reqwest::header::LINK)
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(FeedError::Provider(format!(
                    "github {op_name}: {status} body={body}"
                )));
            }
            let page: Vec<Value> = resp
                .json()
                .await
                .map_err(|e| FeedError::Provider(format!("github {op_name} parse: {e}")))?;
            out.extend(page);
            pages_fetched += 1;
            if let Some(link) = link_header {
                next_path = parse_link_next_path(&link);
            }
        }
        Ok(out)
    }
}

/// Drop PR rows from a mixed-issues array. GitHub's
/// `/repos/{owner}/{repo}/issues` endpoint returns both issues and
/// PRs; PRs are identified by the presence of a `pull_request` field.
pub fn strip_pr_rows(rows: Vec<Value>) -> Vec<Value> {
    rows.into_iter()
        .filter(|v| v.get("pull_request").is_none())
        .collect()
}

/// Filter rows whose `updated_at` is earlier than `floor`. Rows with
/// a missing/malformed `updated_at` are kept (defensive — better to
/// over-report than silently drop).
pub fn filter_by_updated_at(rows: Vec<Value>, floor: Option<DateTime<Utc>>) -> Vec<Value> {
    let Some(floor) = floor else {
        return rows;
    };
    rows.into_iter()
        .filter(|v| {
            v.get("updated_at")
                .and_then(|t| t.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|d| d.with_timezone(&Utc) >= floor)
                .unwrap_or(true)
        })
        .collect()
}

/// Parse a GitHub `Link` header for the `rel="next"` target and
/// return its path+query suffix (i.e. strip `https://api.github.com`
/// so the client's per-request base join still works).
pub fn parse_link_next_path(header: &str) -> Option<String> {
    for part in header.split(',') {
        let part = part.trim();
        // Format: <url>; rel="next"
        let semicolon = part.find(';')?;
        let (url_part, rel_part) = part.split_at(semicolon);
        let url_trimmed = url_part
            .trim()
            .trim_start_matches('<')
            .trim_end_matches('>');
        if rel_part.contains("rel=\"next\"") {
            // Strip the base — everything before the path segment.
            return Some(
                url_trimmed
                    .trim_start_matches("https://api.github.com")
                    .to_string(),
            );
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_header_extracts_next_path() {
        let h = r#"<https://api.github.com/notifications?per_page=50&page=2>; rel="next", <https://api.github.com/notifications?per_page=50&page=3>; rel="last""#;
        assert_eq!(
            parse_link_next_path(h),
            Some("/notifications?per_page=50&page=2".to_string())
        );
    }

    #[test]
    fn link_header_with_only_last_returns_none() {
        let h = r#"<https://api.github.com/notifications?per_page=50&page=3>; rel="last""#;
        assert!(parse_link_next_path(h).is_none());
    }

    #[test]
    fn link_header_empty_returns_none() {
        assert!(parse_link_next_path("").is_none());
    }

    #[test]
    fn strip_pr_rows_drops_rows_with_pull_request_field() {
        let rows = vec![
            serde_json::json!({"number": 1, "title": "issue one"}),
            serde_json::json!({"number": 2, "title": "pr one", "pull_request": {"url": "x"}}),
            serde_json::json!({"number": 3, "title": "issue two"}),
        ];
        let filtered = strip_pr_rows(rows);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0]["number"], 1);
        assert_eq!(filtered[1]["number"], 3);
    }

    #[test]
    fn strip_pr_rows_keeps_everything_when_no_prs() {
        let rows = vec![
            serde_json::json!({"number": 1}),
            serde_json::json!({"number": 2}),
        ];
        assert_eq!(strip_pr_rows(rows).len(), 2);
    }

    #[test]
    fn filter_by_updated_at_keeps_rows_at_or_after_floor() {
        let rows = vec![
            serde_json::json!({"updated_at": "2026-05-10T00:00:00Z", "number": 1}),
            serde_json::json!({"updated_at": "2026-05-18T00:00:00Z", "number": 2}),
            serde_json::json!({"updated_at": "2026-05-15T00:00:00Z", "number": 3}),
        ];
        let floor: DateTime<Utc> = "2026-05-15T00:00:00Z".parse().unwrap();
        let filtered = filter_by_updated_at(rows, Some(floor));
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0]["number"], 2);
        assert_eq!(filtered[1]["number"], 3);
    }

    #[test]
    fn filter_by_updated_at_with_none_floor_keeps_all() {
        let rows = vec![
            serde_json::json!({"updated_at": "2026-05-10T00:00:00Z"}),
            serde_json::json!({"updated_at": "2026-05-18T00:00:00Z"}),
        ];
        assert_eq!(filter_by_updated_at(rows, None).len(), 2);
    }

    #[test]
    fn filter_by_updated_at_keeps_rows_missing_updated_at() {
        // Defensive — better to over-report than silently drop a row
        // GitHub mangled.
        let rows = vec![
            serde_json::json!({"number": 1}),
            serde_json::json!({"updated_at": "garbage", "number": 2}),
            serde_json::json!({"updated_at": "2026-05-18T00:00:00Z", "number": 3}),
        ];
        let floor: DateTime<Utc> = "2026-05-15T00:00:00Z".parse().unwrap();
        let filtered = filter_by_updated_at(rows, Some(floor));
        assert_eq!(filtered.len(), 3);
    }
}
