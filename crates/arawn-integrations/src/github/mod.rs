//! GitHub App integration (I-0045 T-0317).
//!
//! Unlike the OAuth-app-style integrations in this crate (Gmail,
//! Calendar, Drive, Slack), GitHub uses the **App** model: the user
//! installs a per-org app, GitHub redirects to our local callback
//! with an `installation_id`, and we mint short-lived
//! installation-access-tokens on demand by signing a JWT with the
//! app's RSA private key.
//!
//! Provides:
//! - [`GithubIntegration`] — implements [`crate::Integration`] for
//!   the install-app lifecycle.
//! - [`GithubClient`] — auto-refreshing wrapper that exposes an
//!   authenticated `reqwest::Client` with rate-limit handling.
//! - [`GithubAppConfig`] — operator-supplied app credentials (app_id,
//!   private key, slug).
//!
//! Setup instructions for the operator live in
//! `docs/src/integrations/github.md` (created at T-0317 land time).

mod client;
mod install_flow;
mod integration;

pub use client::{GithubClient, InstallationAccessToken};
pub use install_flow::{GithubInstallOutcome, run_install_flow};
pub use integration::{GithubAppConfig, GithubCredentials, GithubIntegration, SERVICE_NAME};
