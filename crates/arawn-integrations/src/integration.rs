use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::IntegrationError;

/// Lifecycle contract every external integration implements.
///
/// Tools that consume an integration (e.g. `gmail_inbox_read`) take an
/// `Arc<dyn Integration>` (or a concrete `Arc<GmailIntegration>` for type
/// safety) at construction time, exactly the way memory tools take an
/// `Arc<MemoryManager>`. The trait does not expose any tool surface — it's
/// strictly about connection lifecycle.
///
/// Implementations are constructed once at server startup and registered
/// into the `LocalService` integration registry. Per ARAWN-A-0001, only
/// the connection lifecycle goes through this trait; per-service operations
/// (read inbox, send mail, etc.) live on the concrete impl and are
/// reached by tools that hold a typed Arc.
#[async_trait]
pub trait Integration: Send + Sync {
    /// Stable service name. Lowercase + snake_case ("gmail",
    /// "google_calendar", "slack"). Used as the dictionary key in the
    /// integration registry, the per-service credential subdirectory name,
    /// and the user-typed argument to `/connect <service>`.
    fn name(&self) -> &str;

    /// Cheap check: are credentials present on disk and (probably) valid?
    /// Does NOT round-trip to the provider — read-from-disk only. A `true`
    /// response means "we have credentials, calls should work"; a real call
    /// can still fail (token revoked server-side, etc.).
    async fn is_connected(&self) -> bool;

    /// Drive whatever credential acquisition flow this integration needs
    /// to acquire credentials and persist them. For OAuth integrations,
    /// this typically means running `oauth_flow::run_oauth_flow`. For
    /// manual-credential integrations (Slack webhook), it might prompt
    /// the user via the supplied `ConnectContext`.
    ///
    /// Returns when credentials have been stored. Errors propagate to
    /// the caller (which will broadcast a `ServerNotice` either way).
    async fn connect(&self, ctx: &dyn ConnectContext) -> Result<(), IntegrationError>;

    /// Drop stored credentials. Idempotent — disconnecting an already-
    /// disconnected integration is not an error.
    async fn disconnect(&self) -> Result<(), IntegrationError>;
}

/// Hooks an `Integration::connect` impl needs from its caller (the server).
/// Lets the integration:
///   - publish the auth URL it wants the user to open
///   - send progress updates (e.g. "waiting for browser…")
///
/// Implemented by `LocalService`'s integration handler. Tests use a mock.
#[async_trait]
pub trait ConnectContext: Send + Sync {
    /// Service name being connected — useful for log/notice formatting.
    fn service(&self) -> &str;

    /// Publish a URL the user should open to complete the flow. The server
    /// is expected to forward this to the TUI (which then attempts to open
    /// it via `open` / `xdg-open`).
    async fn publish_auth_url(&self, url: &url::Url);

    /// Publish a free-form progress note (e.g. "waiting for callback…").
    async fn publish_progress(&self, message: &str);
}

/// Snapshot of one integration's state, returned by `list_integrations` RPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationStatus {
    pub name: String,
    pub connected: bool,
}
