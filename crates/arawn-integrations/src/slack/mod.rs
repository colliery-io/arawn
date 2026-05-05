//! Slack integration. Read/write — agent can browse channel history,
//! post messages, and react. Mirrors the Gmail/Calendar shape:
//!
//! - [`SlackIntegration`] implements [`crate::Integration`].
//! - [`SlackContext`] bundles the slack-morphism client + token for tools.
//! - Six [`arawn_tool::Tool`] impls: `list_channels`, `history`, `post`,
//!   `react`, plus `users_list` (resolve user IDs to names + emails) and
//!   `open_dm` (find/open a DM channel by user id). Cross-channel
//!   `slack_search` is deferred — slack-morphism doesn't typed-expose
//!   `search.messages`; per-channel `slack_history` covers most "what was
//!   discussed" use cases for v1.
//!
//! See `docs/src/integrations/slack.md` for setup and ARAWN-A-0001 § 4
//! for the design call (full OAuth, not webhook).

mod client;
mod integration;
mod tools;

pub use client::{SlackContext, build_slack_client};
pub use integration::{SLACK_OAUTH_SCOPES, SlackIntegration, SlackProviderConfig};
pub use tools::{
    SlackHistoryTool, SlackListChannelsTool, SlackOpenDmTool, SlackPostTool, SlackReactTool,
    SlackUsersListTool,
};
