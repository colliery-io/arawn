//! GitHub — what feeds need from GitHub, plus the production
//! adapter over `arawn-integrations::github`.
//!
//! Templates depend on the [`GithubFeedClient`] trait. Tests fake it
//! externally; production wires [`RealGithubClient`], which holds a
//! `GithubIntegration` and mints installation-access-tokens on demand
//! (see `arawn_integrations::github::GithubClient`).
//!
//! Surface stays small — just the GitHub REST calls the three I-0045
//! feed templates need:
//!
//! - `list_notifications` for `github/notifications` (T-0319).
//! - `search_issues_and_prs` for `github/issues-and-prs` (T-0320).
//! - `search_review_requests` for `github/review-queue` (T-0321).

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
        let url_trimmed = url_part.trim().trim_start_matches('<').trim_end_matches('>');
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
}
