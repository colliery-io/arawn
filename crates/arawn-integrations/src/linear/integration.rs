//! `LinearIntegration` — a personal API key in the encrypted credential
//! store, and a [`LinearClient`] per call.
//!
//! The key comes in through `arawn setup linear`, which checks it with
//! the `viewer` query and saves it here. The integration reads the store
//! on every call, so a new or replaced key works without a restart.
//! There is no runtime env-var fallback: the encrypted entry is the only
//! source (NFR-001).

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::credential_store::CredentialStore;
use crate::error::IntegrationError;
use crate::integration::{ConnectContext, Integration};

use super::client::{HttpTransport, LinearClient, LinearTransport};

/// Stable service name: registry key, credential-store service and
/// entry, and the argument to `arawn setup linear`.
pub const SERVICE_NAME: &str = "linear";

/// What `arawn setup linear` saves: the key, and who it belongs to.
#[derive(Clone, Serialize, Deserialize)]
pub struct LinearCredentials {
    pub api_key: String,
    pub viewer_id: String,
    pub viewer_name: String,
    /// The workspace (Linear "organization") the key belongs to.
    pub organization: String,
}

impl std::fmt::Debug for LinearCredentials {
    // Never print the key.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinearCredentials")
            .field("viewer_id", &self.viewer_id)
            .field("viewer_name", &self.viewer_name)
            .field("organization", &self.organization)
            .finish_non_exhaustive()
    }
}

/// Linear integration handle. Constructed once at server startup, always
/// (no client registration is needed).
pub struct LinearIntegration {
    data_dir: PathBuf,
    transport: Arc<dyn LinearTransport>,
}

impl LinearIntegration {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            transport: Arc::new(HttpTransport::default()),
        }
    }

    /// Replace the HTTP hop (tests).
    pub fn with_transport(mut self, transport: Arc<dyn LinearTransport>) -> Self {
        self.transport = transport;
        self
    }

    pub fn load_credentials(&self) -> Result<Option<LinearCredentials>, IntegrationError> {
        self.credential_store()?.load(SERVICE_NAME)
    }

    /// Save (or replace) the key.
    pub fn save_credentials(&self, creds: &LinearCredentials) -> Result<(), IntegrationError> {
        self.credential_store()?.save(SERVICE_NAME, creds)
    }

    /// Check a key with the `viewer` query. Gives the credentials to
    /// save; saves nothing.
    pub async fn verify_key(&self, api_key: &str) -> Result<LinearCredentials, IntegrationError> {
        let viewer = LinearClient::with_transport(api_key, Arc::clone(&self.transport))
            .viewer()
            .await?;
        Ok(LinearCredentials {
            api_key: api_key.to_string(),
            viewer_id: viewer.id,
            viewer_name: viewer.name,
            organization: viewer.organization.name,
        })
    }

    /// A client for the stored key.
    pub fn client(&self) -> Result<LinearClient, IntegrationError> {
        let creds = self.load_credentials()?.ok_or_else(|| {
            IntegrationError::Provider("linear is not set up — run: arawn setup linear".into())
        })?;
        Ok(LinearClient::with_transport(
            creds.api_key,
            Arc::clone(&self.transport),
        ))
    }

    fn credential_store(&self) -> Result<CredentialStore<LinearCredentials>, IntegrationError> {
        CredentialStore::open(&self.data_dir, SERVICE_NAME)
    }
}

#[async_trait]
impl Integration for LinearIntegration {
    fn name(&self) -> &str {
        SERVICE_NAME
    }

    async fn is_connected(&self) -> bool {
        self.load_credentials().ok().flatten().is_some()
    }

    /// There is no browser flow, and `ConnectContext` cannot take a
    /// secret: point at `arawn setup linear` and fail, so no caller
    /// waits for an auth URL.
    async fn connect(&self, ctx: &dyn ConnectContext) -> Result<(), IntegrationError> {
        let msg = "Linear uses a personal API key, not a browser sign-in. Run: arawn setup linear";
        ctx.publish_progress(msg).await;
        Err(IntegrationError::Provider(msg.into()))
    }

    async fn disconnect(&self) -> Result<(), IntegrationError> {
        self.credential_store()?.delete(SERVICE_NAME)
    }

    async fn capabilities_summary(&self) -> Option<String> {
        let creds = self.load_credentials().ok().flatten()?;
        Some(format!(
            "linear (connected as {} in workspace {}; issues assigned to or created by me — \
             list, get, comment, set state, set assignee, set priority)",
            creds.viewer_name, creds.organization
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::client::tests::{FakeTransport, viewer_body};
    use super::*;
    use tempfile::TempDir;

    struct NullCtx(std::sync::Mutex<Vec<String>>);

    #[async_trait]
    impl ConnectContext for NullCtx {
        fn service(&self) -> &str {
            SERVICE_NAME
        }
        async fn publish_auth_url(&self, _url: &url::Url) {
            panic!("linear must not publish an auth URL");
        }
        async fn publish_progress(&self, message: &str) {
            self.0.lock().unwrap().push(message.to_string());
        }
    }

    #[tokio::test]
    async fn verify_save_connect_disconnect() {
        let tmp = TempDir::new().unwrap();
        let l = LinearIntegration::new(tmp.path().to_path_buf())
            .with_transport(FakeTransport::ok(&[viewer_body()]));
        assert!(!l.is_connected().await);
        assert!(l.capabilities_summary().await.is_none());

        let creds = l.verify_key("lin_api_secret").await.unwrap();
        assert_eq!(creds.viewer_name, "Ada");
        assert!(!format!("{creds:?}").contains("lin_api_secret"));
        l.save_credentials(&creds).unwrap();
        assert!(l.is_connected().await);
        let s = l.capabilities_summary().await.unwrap();
        assert!(s.contains("Ada") && s.contains("Acme"), "{s}");

        l.disconnect().await.unwrap();
        assert!(!l.is_connected().await);
        assert!(l.client().is_err());
    }

    #[tokio::test]
    async fn connect_points_at_setup_and_fails() {
        let tmp = TempDir::new().unwrap();
        let l = LinearIntegration::new(tmp.path().to_path_buf());
        let ctx = NullCtx(Default::default());
        let err = l.connect(&ctx).await.unwrap_err();
        assert!(err.to_string().contains("arawn setup linear"));
        assert!(ctx.0.lock().unwrap()[0].contains("arawn setup linear"));
    }
}
