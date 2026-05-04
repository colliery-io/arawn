//! External-service integrations (Gmail, Calendar, Slack, ...).
//!
//! Provides three things to the rest of arawn:
//!
//! - The [`Integration`] trait — connection lifecycle (`name`, `is_connected`,
//!   `connect`, `disconnect`). Tools that depend on an integration look it up
//!   by name in the `LocalService::integration_registry` and use whatever
//!   provider-specific client that integration exposes.
//!
//! - [`CredentialStore`] — encrypted-at-rest storage for non-OAuth credentials
//!   (e.g. webhook URLs). OAuth tokens use [`arawn_auth::TokenStore`] directly,
//!   which is purpose-built for them.
//!
//! - [`oauth_flow::run_oauth_flow`] — composes [`arawn_auth::OAuthClient`] +
//!   [`arawn_auth::CallbackServer`] into the standard browser-based dance every
//!   integration needs. Provider config is opaque; each integration supplies
//!   its own URLs, scopes, and credentials.
//!
//! Per [ARAWN-A-0001](../../.metis/adrs/ARAWN-A-0001.md), credentials live
//! under `<data_dir>/integrations/<service>/`, encrypted with the same
//! ChaCha20Poly1305 + per-data-dir master key that `TokenStore` uses.

pub mod calendar;
pub mod credential_store;
pub mod error;
pub mod gmail;
pub mod google_common;
pub mod integration;
pub mod oauth_flow;
pub mod slack;

pub use credential_store::CredentialStore;
pub use error::IntegrationError;
pub use integration::{ConnectContext, Integration, IntegrationStatus};
pub use oauth_flow::{run_oauth_flow, OAuthOutcome};

/// Install rustls' `ring` crypto provider as the process default. Must be
/// called once at server startup, before any integration constructs a
/// hyper-rustls connector — slack-morphism, the Google API hubs, and
/// reqwest with rustls-tls all share rustls' process-level provider slot.
///
/// Idempotent: a second call is silently a no-op. Returning `()` keeps
/// the call site clean (no `.ok()` ceremony).
pub fn install_default_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}
