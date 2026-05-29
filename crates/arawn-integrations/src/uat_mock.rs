//! ARAWN-I-0062 T-A: lifecycle-level mock `Integration` for UAT.
//!
//! `UatMockIntegration` reports `is_connected() == true` for an arbitrary
//! service name. Its job is narrow: make `LocalService::connected_services`
//! list the service so the engine's `filter_tools_for_context` includes that
//! category. It does **not** by itself wire up provider clients — the actual
//! `gmail_*` / `calendar_*` / `slack_*` tools take typed `Arc<...Integration>`
//! references at construction, so making them callable in UAT additionally
//! requires per-service mock clients (see follow-up tasks T-B/C/D).
//!
//! This is **not** behind a feature flag. The wiring on the server side is
//! gated by the `ARAWN_UAT_MOCK_INTEGRATIONS` env var, so production runs
//! never construct one of these unless an operator explicitly opts in.

use async_trait::async_trait;

use crate::error::IntegrationError;
use crate::integration::{ConnectContext, Integration};

/// Always-connected, no-op `Integration` used by the UAT harness to flip the
/// visibility check for a service category. Lifecycle-only — does not produce
/// a provider client.
pub struct UatMockIntegration {
    name: String,
}

impl UatMockIntegration {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl Integration for UatMockIntegration {
    fn name(&self) -> &str {
        &self.name
    }

    async fn is_connected(&self) -> bool {
        true
    }

    async fn connect(&self, _ctx: &dyn ConnectContext) -> Result<(), IntegrationError> {
        // No real flow — already "connected" from the moment it's registered.
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), IntegrationError> {
        // No credentials to drop. Idempotent no-op.
        Ok(())
    }

    async fn capabilities_summary(&self) -> Option<String> {
        Some(format!("{} (uat-mock; connected)", self.name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoopCtx;

    #[async_trait]
    impl ConnectContext for NoopCtx {
        fn service(&self) -> &str {
            "noop"
        }
        async fn publish_auth_url(&self, _url: &url::Url) {}
        async fn publish_progress(&self, _message: &str) {}
    }

    #[tokio::test]
    async fn reports_connected_and_named() {
        let mock = UatMockIntegration::new("gmail");
        assert_eq!(mock.name(), "gmail");
        assert!(mock.is_connected().await);
        mock.connect(&NoopCtx).await.unwrap();
        mock.disconnect().await.unwrap();
        let summary = mock.capabilities_summary().await.unwrap();
        assert!(summary.contains("gmail") && summary.contains("uat-mock"));
    }
}
