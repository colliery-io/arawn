//! `GithubIntegration` — wires up the install flow + credential
//! storage + per-call `GithubClient` construction (I-0045 T-0317).
//!
//! Connection state for GitHub is split:
//!
//! * **Operator-supplied** (in arawn.toml or env): `app_id`,
//!   `private_key_pem` (or path), `app_slug`. These survive in
//!   `GithubAppConfig` on the integration struct.
//! * **Per-user, captured at install time**: `installation_id`
//!   (and `setup_action` we record for audit). These live in the
//!   encrypted credential store keyed by `github`.
//!
//! `is_connected()` is a cheap disk read: does a credential row
//! exist? `connect()` runs the install flow + persists; `disconnect`
//! deletes the row.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::credential_store::CredentialStore;
use crate::error::IntegrationError;
use crate::integration::{ConnectContext, Integration};

use super::client::GithubClient;
use super::install_flow::run_install_flow;

/// Stable service name. Used as the credential-store key, integration
/// registry key, and argument to `/integrations connect github`.
pub const SERVICE_NAME: &str = "github";

/// Operator-supplied GitHub App credentials. Loaded once at startup
/// (typically from `[integrations.github]` in arawn.toml + env for
/// the PEM body). Same struct lives for the daemon's life — install
/// per-user state is separate (see [`GithubCredentials`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubAppConfig {
    /// Numeric App ID from the GitHub App settings page.
    pub app_id: String,
    /// RSA private key in PEM format (`-----BEGIN RSA PRIVATE KEY-----…`).
    pub private_key_pem: String,
    /// App slug — the URL-segment GitHub assigns. Used to build the
    /// public install URL.
    pub app_slug: String,
}

/// Per-user install state, persisted encrypted at rest via
/// `CredentialStore`. Created on `connect()` and consumed on every
/// API call to mint short-lived access tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubCredentials {
    pub installation_id: u64,
    /// What GitHub said the user did on the install page —
    /// `install` (fresh install) or `update` (changed repo scope on
    /// an existing install).
    pub setup_action: String,
}

/// GitHub App integration handle. Constructed once at server
/// startup; tools (downstream tasks) take an `Arc<GithubIntegration>`.
pub struct GithubIntegration {
    data_dir: PathBuf,
    app: GithubAppConfig,
}

impl GithubIntegration {
    pub fn new(data_dir: PathBuf, app: GithubAppConfig) -> Self {
        Self { data_dir, app }
    }

    /// Public install URL for this app:
    /// `https://github.com/apps/<slug>/installations/new`.
    pub fn install_url(&self) -> Result<Url, IntegrationError> {
        Url::parse(&format!(
            "https://github.com/apps/{}/installations/new",
            self.app.app_slug
        ))
        .map_err(|e| IntegrationError::Provider(format!("install URL: {e}")))
    }

    /// Load the persisted install row, if any.
    pub fn load_credentials(&self) -> Result<Option<GithubCredentials>, IntegrationError> {
        let store = self.credential_store()?;
        store.load(SERVICE_NAME)
    }

    /// Build an authenticated client for tools and feed templates.
    /// Returns `NotConnected`-flavoured error when no install row
    /// has been persisted yet.
    pub fn client(&self) -> Result<Arc<GithubClient>, IntegrationError> {
        let creds = self.load_credentials()?.ok_or_else(|| {
            IntegrationError::Provider(
                "github not connected — run /integrations connect github".into(),
            )
        })?;
        Ok(Arc::new(GithubClient::new(
            self.app.clone(),
            creds.installation_id,
        )?))
    }

    fn credential_store(&self) -> Result<CredentialStore<GithubCredentials>, IntegrationError> {
        CredentialStore::open(&self.data_dir, SERVICE_NAME)
    }
}

#[async_trait]
impl Integration for GithubIntegration {
    fn name(&self) -> &str {
        SERVICE_NAME
    }

    async fn is_connected(&self) -> bool {
        // Disk-only check per the trait contract.
        match self.credential_store() {
            Ok(store) => store.load(SERVICE_NAME).ok().flatten().is_some(),
            Err(_) => false,
        }
    }

    async fn connect(&self, ctx: &dyn ConnectContext) -> Result<(), IntegrationError> {
        let url = self.install_url()?;
        let outcome = run_install_flow(url, "/oauth/callback", ctx).await?;
        let store = self.credential_store()?;
        store.save(
            SERVICE_NAME,
            &GithubCredentials {
                installation_id: outcome.installation_id,
                setup_action: outcome.setup_action,
            },
        )?;
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), IntegrationError> {
        let store = self.credential_store()?;
        store.delete(SERVICE_NAME)?;
        Ok(())
    }

    async fn capabilities_summary(&self) -> Option<String> {
        if self.is_connected().await {
            Some(format!(
                "github (connected as app `{}`; read-only — notifications, issues, PRs, review queue)",
                self.app.app_slug
            ))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_app() -> GithubAppConfig {
        GithubAppConfig {
            app_id: "12345".into(),
            private_key_pem: "-- placeholder for non-network tests --".into(),
            app_slug: "arawn-test".into(),
        }
    }

    #[test]
    fn install_url_is_public_app_page() {
        let tmp = TempDir::new().unwrap();
        let g = GithubIntegration::new(tmp.path().to_path_buf(), make_app());
        let url = g.install_url().unwrap();
        assert_eq!(
            url.as_str(),
            "https://github.com/apps/arawn-test/installations/new"
        );
    }

    #[tokio::test]
    async fn is_connected_starts_false() {
        let tmp = TempDir::new().unwrap();
        let g = GithubIntegration::new(tmp.path().to_path_buf(), make_app());
        assert!(!g.is_connected().await);
    }

    #[tokio::test]
    async fn save_then_load_round_trips() {
        let tmp = TempDir::new().unwrap();
        let g = GithubIntegration::new(tmp.path().to_path_buf(), make_app());
        let store = g.credential_store().unwrap();
        store
            .save(
                SERVICE_NAME,
                &GithubCredentials {
                    installation_id: 42,
                    setup_action: "install".into(),
                },
            )
            .unwrap();
        assert!(g.is_connected().await);
        let loaded = g.load_credentials().unwrap().unwrap();
        assert_eq!(loaded.installation_id, 42);
        assert_eq!(loaded.setup_action, "install");
    }

    #[tokio::test]
    async fn disconnect_clears_credentials() {
        let tmp = TempDir::new().unwrap();
        let g = GithubIntegration::new(tmp.path().to_path_buf(), make_app());
        let store = g.credential_store().unwrap();
        store
            .save(
                SERVICE_NAME,
                &GithubCredentials {
                    installation_id: 1,
                    setup_action: "install".into(),
                },
            )
            .unwrap();
        g.disconnect().await.unwrap();
        assert!(!g.is_connected().await);
    }

    #[tokio::test]
    async fn capabilities_summary_only_when_connected() {
        let tmp = TempDir::new().unwrap();
        let g = GithubIntegration::new(tmp.path().to_path_buf(), make_app());
        assert!(g.capabilities_summary().await.is_none());
        let store = g.credential_store().unwrap();
        store
            .save(
                SERVICE_NAME,
                &GithubCredentials {
                    installation_id: 1,
                    setup_action: "install".into(),
                },
            )
            .unwrap();
        let s = g.capabilities_summary().await.unwrap();
        assert!(s.contains("connected"));
        assert!(s.contains("arawn-test"));
    }
}
