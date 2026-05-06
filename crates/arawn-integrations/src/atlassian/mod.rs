//! Atlassian integration covering both Jira and Confluence Cloud.
//!
//! One OAuth dance, one client_id/secret, one persisted token; both tool
//! families light up after `/connect atlassian`. Built against Atlassian
//! Cloud's REST APIs directly (no Rust SDK in the gold-standard family
//! exists for Atlassian).
//!
//! - [`AtlassianIntegration`] implements [`crate::Integration`]. OAuth via
//!   `https://auth.atlassian.com`; post-token, calls
//!   `oauth/token/accessible-resources` to discover the user's
//!   `cloud_id`s and persists the list in the token's `extras` field.
//! - [`AtlassianClient`] is a small HTTP wrapper that substitutes
//!   `cloud_id` into the API base URL and refreshes the access token
//!   on demand (Atlassian tokens are 1-hour-lived).
//! - Six Jira tools and five Confluence tools, each declaring its
//!   required scopes for the runtime check pattern from T-0204.
//!
//! See `docs/src/integrations/atlassian.md` for setup.

mod client;
mod confluence;
mod integration;
mod jira;

pub use client::AtlassianClient;
pub use confluence::{
    ConfluenceCreatePageTool, ConfluenceGetPageTool, ConfluenceListSpacesTool,
    ConfluenceSearchTool, ConfluenceUpdatePageTool,
};
pub use integration::{
    ATLASSIAN_OAUTH_SCOPES, AtlassianIntegration, AtlassianProviderConfig,
    AtlassianSite, DEFAULT_ATLASSIAN_REDIRECT_PORT,
};
pub use jira::{
    JiraAddCommentTool, JiraCreateIssueTool, JiraGetIssueTool, JiraSearchTool,
    JiraTransitionIssueTool, JiraUpdateIssueTool,
};
