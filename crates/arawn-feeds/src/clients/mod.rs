//! Per-provider client traits — the mock-injection seam.
//!
//! Each `clients/<provider>.rs` owns:
//!
//! - The `*FeedClient` trait describing exactly the upstream calls
//!   feeds need from that integration. Kept small on purpose; only
//!   methods at least one template actually uses.
//! - The production `Real*Client` adapter wrapping the matching
//!   `arawn-integrations` client.
//!
//! Test fakes live external to this crate, in `tests/` files that
//! impl the trait directly with whatever canned behavior the test
//! needs.
//!
//! Templates depend on the trait, never on `arawn-integrations` or
//! `slack-morphism` directly — keeps templates mock-testable.

use std::sync::Arc;

pub mod slack;

pub use slack::{
    ChannelKind, RealSlackClient, SlackAuthInfo, SlackFeedClient, SlackHistoryPage,
    classify_channel_id,
};

/// Bundle of every provider client a template might want to use.
/// Implementors return `Some(...)` for providers configured by the
/// user and `None` for ones they're not connected to.
pub trait FeedClients: Send + Sync {
    fn slack(&self) -> Option<Arc<dyn SlackFeedClient>>;
}

/// No-op `FeedClients`: every provider returns `None`. Useful for
/// stub templates that don't need any provider, and as a safe default
/// in tests when a template under test only uses one provider.
pub struct NoopClients;

impl FeedClients for NoopClients {
    fn slack(&self) -> Option<Arc<dyn SlackFeedClient>> {
        None
    }
}

/// Production bundle. Built at server boot from the integrations the
/// user has connected. Use the `with_*` builders to wire each
/// provider's adapter.
#[derive(Default)]
pub struct RealClients {
    slack: Option<Arc<dyn SlackFeedClient>>,
}

impl RealClients {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_slack(
        mut self,
        integration: Arc<arawn_integrations::slack::SlackIntegration>,
    ) -> Self {
        self.slack = Some(Arc::new(RealSlackClient::new(integration)));
        self
    }
}

impl FeedClients for RealClients {
    fn slack(&self) -> Option<Arc<dyn SlackFeedClient>> {
        self.slack.clone()
    }
}
