//! Linear integration (COLLIERY-I-0532).
//!
//! Linear is GraphQL-only and, for one person in one workspace, a
//! personal API key is enough: no OAuth app, no callback server.
//!
//! - [`LinearIntegration`] implements [`crate::Integration`]. It is
//!   connected when the encrypted [`crate::CredentialStore`] holds the
//!   `linear` entry, which `arawn setup linear` writes after it checks
//!   the key with the `viewer` query. `connect()` cannot take a secret,
//!   so it only points at `arawn setup linear`.
//! - [`LinearClient`] is a hand-rolled GraphQL client: one POST per
//!   named document from [`queries`], errors mapped to
//!   [`crate::IntegrationError`] (`RATELIMITED` → `RateLimited`).
//! - Six agent tools ([`linear_tools`]): list and get, then comment,
//!   set state, set assignee and set priority.
//!
//! See `docs/src/content/docs/how-to/connect-linear.md` for setup.

mod client;
mod integration;
pub mod models;
pub mod queries;
mod tools;
mod uat_tools;

pub use client::{
    GraphqlReply, HttpTransport, LINEAR_API_URL, LinearClient, LinearTransport, Viewer,
    my_issues_filter,
};
pub use integration::{LinearCredentials, LinearIntegration, SERVICE_NAME};
pub use models::{Issue, IssuePage};
pub use tools::{
    LinearCommentTool, LinearGetIssueTool, LinearListMyIssuesTool, LinearSetAssigneeTool,
    LinearSetPriorityTool, LinearSetStateTool, linear_tools,
};
pub use uat_tools::uat_linear_tools;
