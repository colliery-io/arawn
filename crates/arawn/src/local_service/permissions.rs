//! `LocalService` inherent methods backing the `permissions.*` portion of
//! `ArawnService`. The trait shell in `super::mod` delegates to these.

use crate::lock_ext::Recover;
use arawn_service::{PermissionModeInfo, ServiceError};
use tracing::info;

use super::LocalService;

impl LocalService {
    pub(super) async fn get_permission_mode_inner(
        &self,
    ) -> Result<PermissionModeInfo, ServiceError> {
        let mode = *self.permission_mode.read().recover();
        Ok(PermissionModeInfo {
            mode: serde_json::to_value(mode)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| format!("{mode:?}")),
        })
    }

    pub(super) async fn set_permission_mode_inner(
        &self,
        mode_str: &str,
    ) -> Result<PermissionModeInfo, ServiceError> {
        let mode: arawn_engine::permissions::PermissionMode =
            serde_json::from_value(serde_json::json!(mode_str)).map_err(|_| {
                ServiceError::InvalidOperation(format!(
                    "unknown mode '{mode_str}'. Valid: ask, edits, full, plan"
                ))
            })?;
        *self.permission_mode.write().recover() = mode;
        info!(mode = %mode_str, "permission mode updated");
        Ok(PermissionModeInfo {
            mode: mode_str.to_string(),
        })
    }

    pub(super) async fn get_capabilities_inner(
        &self,
    ) -> Result<arawn_service::ServerCapabilities, ServiceError> {
        let embeddings_available = self
            .memory_manager
            .as_ref()
            .map(|m| m.embedder().is_some())
            .unwrap_or(false);
        Ok(arawn_service::ServerCapabilities {
            server_version: env!("CARGO_PKG_VERSION").to_string(),
            embeddings_available,
        })
    }

    pub(super) async fn get_permissions_status_inner(
        &self,
    ) -> Result<arawn_service::PermissionsStatus, ServiceError> {
        use arawn_engine::permissions::{PermissionDecision, RuleKind};

        let rules = self.permission_rules.read().recover().clone();
        let mode = *self.permission_mode.read().recover();
        let mode_str = format!("{mode:?}").to_lowercase();

        let mut allow_rules = Vec::new();
        let mut deny_rules = Vec::new();
        let mut ask_rules = Vec::new();
        for r in &rules {
            let spec = r.display_spec();
            match r.kind {
                RuleKind::Allow => allow_rules.push(spec),
                RuleKind::Deny => deny_rules.push(spec),
                RuleKind::Ask => ask_rules.push(spec),
            }
        }

        let recent_decisions = self
            .permission_audit
            .lock()
            .recover()
            .iter()
            .map(|e| {
                let decision_str = match e.decision {
                    PermissionDecision::Allowed => "allowed",
                    PermissionDecision::Denied => "denied",
                    PermissionDecision::Ask => "ask",
                    PermissionDecision::NoMatch => "no_match",
                };
                let timestamp = chrono::DateTime::<chrono::Utc>::from(e.timestamp).to_rfc3339();
                arawn_service::PermissionAuditEntry {
                    timestamp,
                    tool_name: e.tool_name.clone(),
                    tool_input_summary: e.tool_input_summary.clone(),
                    decision: decision_str.to_string(),
                    reason: e.reason.clone(),
                }
            })
            .collect();

        Ok(arawn_service::PermissionsStatus {
            mode: mode_str,
            allow_rules,
            deny_rules,
            ask_rules,
            recent_decisions,
        })
    }
}
