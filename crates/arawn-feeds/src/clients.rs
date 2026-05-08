//! Per-provider client traits — the mock-injection seam.
//!
//! Templates depend on small per-provider traits (`SlackFeedClient`,
//! `GmailFeedClient`, etc.) that expose only the methods feeds need
//! from each integration. Real impls wrap the existing
//! `arawn-integrations` clients; test impls are fakes that return
//! canned data.
//!
//! `FeedClients` is the bundle. `TemplateCtx::new(Arc<dyn FeedClients>)`
//! is how a template gets at any provider; tests build a fake impl
//! returning whatever's needed.

use std::sync::Arc;

use async_trait::async_trait;

use crate::error::FeedError;

/// Bundle of all provider client traits a template might need.
/// Implementors return `Some(...)` for providers they have configured
/// and `None` for ones they don't.
pub trait FeedClients: Send + Sync {
    fn slack(&self) -> Option<Arc<dyn SlackFeedClient>>;
}

/// What feeds need from Slack. Designed for the `slack/channel-archive`
/// flow: list channel history since a `latest_ts` cursor.
///
/// Kept small on purpose — only the methods feeds actually use. As
/// more Slack templates land, this trait grows but the surface is
/// always feed-driven, not "everything Slack can do."
#[async_trait]
pub trait SlackFeedClient: Send + Sync {
    /// Resolve a channel name (`#design`) or id (`CABCDEF`) to its
    /// channel id. Used at registration time to validate `params`.
    async fn resolve_channel(&self, name_or_id: &str) -> Result<String, FeedError>;

    /// Fetch messages from `channel_id` newer than `oldest_ts`. If
    /// `oldest_ts` is `None`, returns the last 24h. Returns messages
    /// in chronological order with the newest's `ts` as the "next
    /// cursor" suggestion.
    async fn channel_history(
        &self,
        channel_id: &str,
        oldest_ts: Option<&str>,
    ) -> Result<SlackHistoryPage, FeedError>;
}

/// One page of Slack channel history. Templates don't paginate — the
/// client either returns everything since the cursor in one call, or
/// pages internally.
#[derive(Debug, Clone)]
pub struct SlackHistoryPage {
    /// Raw API messages, oldest-first. Each entry is the raw JSON
    /// payload Slack returned — templates write this verbatim to disk
    /// to preserve full fidelity.
    pub messages: Vec<serde_json::Value>,
    /// Slack's `ts` of the newest message in this page, or the prior
    /// cursor if no new messages. The template persists this as the
    /// next cursor.
    pub next_cursor_ts: Option<String>,
}

/// No-op `FeedClients` impl: every provider returns `None`. Useful for
/// stub templates that don't need provider access, and as a safe
/// default in tests where the template under test only uses one
/// provider.
pub struct NoopClients;

impl FeedClients for NoopClients {
    fn slack(&self) -> Option<Arc<dyn SlackFeedClient>> {
        None
    }
}
