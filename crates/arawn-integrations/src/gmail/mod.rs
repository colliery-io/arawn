//! Gmail integration — first concrete consumer of the integration framework.
//!
//! Provides:
//! - [`GmailIntegration`] — implements [`crate::Integration`] for OAuth lifecycle.
//! - [`GmailClient`] — refresh-aware wrapper around the Gmail REST v1 API.
//! - Four [`arawn_tool::Tool`] impls: `gmail_inbox_read`, `gmail_search`,
//!   `gmail_send`, `gmail_mark_read`.
//!
//! See `docs/src/integrations/gmail.md` for the Google Cloud Console
//! setup steps users need to complete before connecting.

mod client;
mod integration;
mod tools;

pub use client::{GmailHub, client_from_token_store};
pub use integration::{GmailIntegration, GmailProviderConfig};
pub use tools::{
    GmailGetMessageTool, GmailInboxReadTool, GmailMarkReadTool, GmailSearchTool, GmailSendTool,
};
