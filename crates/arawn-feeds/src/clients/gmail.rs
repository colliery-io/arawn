//! Gmail — what feeds need from Gmail, plus the production adapter
//! over `arawn-integrations` + `google-gmail1`.
//!
//! Templates depend on the [`GmailFeedClient`] trait. Tests fake it
//! externally; production wires [`RealGmailClient`], which reuses the
//! same `GmailIntegration` (and persisted token) the existing Gmail
//! tools use.
//!
//! Surface intentionally small: list ids matching a query, then fetch
//! each one full. All three Gmail feed templates are query-shaped, so
//! pushing query construction into the templates keeps the trait
//! provider-agnostic and makes mocking trivial.

use std::sync::Arc;

use arawn_integrations::gmail::GmailIntegration;
use async_trait::async_trait;
use serde_json::Value;

use crate::error::FeedError;

/// What feeds need from Gmail.
#[async_trait]
pub trait GmailFeedClient: Send + Sync {
    /// List message ids matching the given Gmail search query, capped
    /// at `max_results`. Order is Gmail-default (most recent first).
    async fn list_message_ids(
        &self,
        query: &str,
        max_results: u32,
    ) -> Result<Vec<String>, FeedError>;

    /// Fetch a full message payload by id. Returns the raw API JSON so
    /// templates preserve full fidelity on disk (headers, parts, body
    /// data). Format used: `full`.
    async fn get_message(&self, id: &str) -> Result<Value, FeedError>;
}

// ─── Production adapter ──────────────────────────────────────────────

pub struct RealGmailClient {
    integration: Arc<GmailIntegration>,
}

impl RealGmailClient {
    pub fn new(integration: Arc<GmailIntegration>) -> Self {
        Self { integration }
    }
}

fn integ_err(e: arawn_integrations::IntegrationError) -> FeedError {
    use arawn_integrations::IntegrationError;
    match e {
        IntegrationError::NotConnected(msg) => FeedError::Auth(msg),
        other => FeedError::Provider(other.user_message()),
    }
}

fn google_err(op: &str, msg: String) -> FeedError {
    if msg.contains("rateLimitExceeded") || msg.contains("userRateLimitExceeded") {
        FeedError::RateLimited { retry_after: None }
    } else if msg.contains("invalid_grant")
        || msg.contains("token_expired")
        || msg.contains("unauthorized_client")
    {
        FeedError::Auth(format!("{op}: {msg}"))
    } else {
        FeedError::Provider(format!("{op}: {msg}"))
    }
}

#[async_trait]
impl GmailFeedClient for RealGmailClient {
    async fn list_message_ids(
        &self,
        query: &str,
        max_results: u32,
    ) -> Result<Vec<String>, FeedError> {
        // Walk Gmail's `nextPageToken` until we have `max_results` ids
        // or the result set is exhausted. Per-page is capped at
        // Gmail's max of 500. For the cron-tick path, callers pass a
        // small `max_results` (~100) and we make a single call. For
        // the backfill path (T-0234), callers pass a larger
        // `max_results` (e.g. 5000) and we paginate until done.
        const GMAIL_MAX_PAGE_SIZE: u32 = 500;
        let hub = self.integration.hub().map_err(integ_err)?;
        let mut collected: Vec<String> = Vec::new();
        let mut page_token: Option<String> = None;
        while collected.len() < max_results as usize {
            let remaining = max_results as usize - collected.len();
            let page_size = (remaining as u32).min(GMAIL_MAX_PAGE_SIZE);
            let mut req = hub
                .users()
                .messages_list("me")
                .q(query)
                .max_results(page_size);
            if let Some(t) = page_token.as_deref() {
                req = req.page_token(t);
            }
            let (_resp, list) = req
                .doit()
                .await
                .map_err(|e| google_err("messages.list", e.to_string()))?;
            collected.extend(
                list.messages
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|m| m.id),
            );
            match list.next_page_token {
                Some(t) if !t.is_empty() => page_token = Some(t),
                _ => break,
            }
        }
        Ok(collected)
    }

    async fn get_message(&self, id: &str) -> Result<Value, FeedError> {
        let hub = self.integration.hub().map_err(integ_err)?;
        let (_resp, message) = hub
            .users()
            .messages_get("me", id)
            .format("full")
            .doit()
            .await
            .map_err(|e| google_err("messages.get", e.to_string()))?;
        serde_json::to_value(message)
            .map_err(|e| FeedError::Schema(format!("serialize Message: {e}")))
    }
}
