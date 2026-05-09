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

pub mod atlassian;
pub mod calendar;
pub mod drive;
pub mod gmail;
pub mod slack;

pub use atlassian::{
    AtlassianFeedClient, ConfluencePageBody, ConfluencePageMeta, ConfluenceSpaceMeta,
    JiraIssueDetail, JiraIssueMeta, JiraProjectMeta, RealAtlassianClient,
};
pub use calendar::{CalendarFeedClient, RealCalendarClient};
pub use drive::{DriveFeedClient, DriveFile, RealDriveClient, export_for, is_unsupported_google_native};
pub use gmail::{GmailFeedClient, RealGmailClient};
pub use slack::{
    ChannelKind, RealSlackClient, SlackAuthInfo, SlackChannel, SlackFeedClient, SlackHistoryPage,
    classify_channel_id,
};

/// Bundle of every provider client a template might want to use.
/// Implementors return `Some(...)` for providers configured by the
/// user and `None` for ones they're not connected to.
pub trait FeedClients: Send + Sync {
    fn slack(&self) -> Option<Arc<dyn SlackFeedClient>>;
    fn calendar(&self) -> Option<Arc<dyn CalendarFeedClient>>;
    fn gmail(&self) -> Option<Arc<dyn GmailFeedClient>>;
    fn drive(&self) -> Option<Arc<dyn DriveFeedClient>>;
    fn atlassian(&self) -> Option<Arc<dyn AtlassianFeedClient>>;
}

/// No-op `FeedClients`: every provider returns `None`. Useful for
/// stub templates that don't need any provider, and as a safe default
/// in tests when a template under test only uses one provider.
pub struct NoopClients;

impl FeedClients for NoopClients {
    fn slack(&self) -> Option<Arc<dyn SlackFeedClient>> {
        None
    }
    fn calendar(&self) -> Option<Arc<dyn CalendarFeedClient>> {
        None
    }
    fn gmail(&self) -> Option<Arc<dyn GmailFeedClient>> {
        None
    }
    fn drive(&self) -> Option<Arc<dyn DriveFeedClient>> {
        None
    }
    fn atlassian(&self) -> Option<Arc<dyn AtlassianFeedClient>> {
        None
    }
}

/// Production bundle. Built at server boot from the integrations the
/// user has connected. Use the `with_*` builders to wire each
/// provider's adapter.
#[derive(Default)]
pub struct RealClients {
    slack: Option<Arc<dyn SlackFeedClient>>,
    calendar: Option<Arc<dyn CalendarFeedClient>>,
    gmail: Option<Arc<dyn GmailFeedClient>>,
    drive: Option<Arc<dyn DriveFeedClient>>,
    atlassian: Option<Arc<dyn AtlassianFeedClient>>,
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

    pub fn with_calendar(
        mut self,
        integration: Arc<arawn_integrations::calendar::GoogleCalendarIntegration>,
    ) -> Self {
        self.calendar = Some(Arc::new(RealCalendarClient::new(integration)));
        self
    }

    pub fn with_gmail(
        mut self,
        integration: Arc<arawn_integrations::gmail::GmailIntegration>,
    ) -> Self {
        self.gmail = Some(Arc::new(RealGmailClient::new(integration)));
        self
    }

    pub fn with_drive(
        mut self,
        integration: Arc<arawn_integrations::drive::GoogleDriveIntegration>,
    ) -> Self {
        self.drive = Some(Arc::new(RealDriveClient::new(integration)));
        self
    }

    pub fn with_atlassian(
        mut self,
        integration: Arc<arawn_integrations::atlassian::AtlassianIntegration>,
    ) -> Self {
        self.atlassian = Some(Arc::new(RealAtlassianClient::new(integration)));
        self
    }
}

impl FeedClients for RealClients {
    fn slack(&self) -> Option<Arc<dyn SlackFeedClient>> {
        self.slack.clone()
    }
    fn calendar(&self) -> Option<Arc<dyn CalendarFeedClient>> {
        self.calendar.clone()
    }
    fn gmail(&self) -> Option<Arc<dyn GmailFeedClient>> {
        self.gmail.clone()
    }
    fn drive(&self) -> Option<Arc<dyn DriveFeedClient>> {
        self.drive.clone()
    }
    fn atlassian(&self) -> Option<Arc<dyn AtlassianFeedClient>> {
        self.atlassian.clone()
    }
}
