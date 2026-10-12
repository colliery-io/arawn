//! Slack integration. Read/write — agent can browse channel history,
//! post messages, and react. Mirrors the Gmail/Calendar shape:
//!
//! - [`SlackApp`] is the one Slack app; [`SlackIntegration`] is one
//!   workspace (`slack@<alias>`) and implements [`crate::Integration`].
//! - [`SlackContext`] bundles the slack-morphism client + token for tools.
//! - Six [`arawn_tool::Tool`] impls per workspace, named
//!   `slack_<alias>_<op>` ([`tool_name`]): `list_channels`, `history`, `post`,
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
pub mod uat_tools;

pub use client::{SlackContext, build_slack_client};
pub use integration::{
    ALIAS_MAX_LEN, DEFAULT_ALIAS, DEFAULT_SLACK_REDIRECT_PORT, SERVICE_NAME, SLACK_OAUTH_SCOPES,
    SLACK_OAUTH_USER_SCOPES, SlackApp, SlackIntegration, SlackProviderConfig, SlackTeam, alias_of,
    connected_aliases, is_valid_alias, sanitize_alias, team_of, workspace_name,
};
pub use tools::{
    SLACK_TOOL_OPS, SlackHistoryTool, SlackListChannelsTool, SlackOpenDmTool, SlackPostTool,
    SlackReactTool, SlackUsersListTool, slack_tool_names, slack_tools, tool_name,
};
pub use uat_tools::{UAT_ALIAS, UatSlackHistoryTool, UatSlackListChannelsTool, uat_slack_tools};
