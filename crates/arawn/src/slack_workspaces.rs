//! Register and drop one Slack workspace (`slack@<alias>`, COLLIERY-I-0520):
//! its integration in the service registry and its tools in the tool
//! registry. Used at startup, and by connect and disconnect, so a
//! workspace comes and goes with no restart.

use crate::lock_ext::Recover;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use arawn_engine::ToolRegistry;
use arawn_integrations::Integration;
use arawn_integrations::slack::{SlackApp, SlackIntegration};
use tracing::{info, warn};

/// The service's integration registry.
pub type IntegrationRegistry = Arc<RwLock<HashMap<String, Arc<dyn Integration>>>>;

/// Register `ws` and its tools. A workspace already registered is
/// replaced.
pub fn register(
    integrations: &IntegrationRegistry,
    tools: &ToolRegistry,
    ws: Arc<SlackIntegration>,
) {
    let name = ws.name().to_string();
    let set = arawn_integrations::slack::slack_tools(Arc::clone(&ws));
    let count = set.len();
    for tool in set {
        tools.register(tool);
    }
    integrations
        .write()
        .recover()
        .insert(name.clone(), ws as Arc<dyn Integration>);
    info!(workspace = %name, tools = count, "Slack workspace registered");
}

/// Register every workspace with a stored token (startup discovery).
/// Returns their aliases.
pub fn register_connected(
    app: &SlackApp,
    integrations: &IntegrationRegistry,
    tools: &ToolRegistry,
) -> Vec<String> {
    let mut out = Vec::new();
    for alias in app.connected_aliases() {
        match app.workspace(&alias) {
            Ok(ws) => {
                register(integrations, tools, Arc::new(ws));
                out.push(alias);
            }
            Err(e) => warn!(alias = %alias, error = %e, "Slack workspace skipped"),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn save_workspace(dir: &std::path::Path, alias: &str) {
        let mut extras = serde_json::Map::new();
        extras.insert(
            "team".into(),
            serde_json::json!({ "id": format!("T-{alias}"), "name": alias }),
        );
        arawn_auth::TokenStore::open(dir)
            .unwrap()
            .save(
                &format!("slack@{alias}"),
                &arawn_auth::Token {
                    access: format!("xoxb-{alias}"),
                    refresh: None,
                    expires_at: None,
                    scope: Some("channels:read,chat:write".into()),
                    token_type: "Bearer".into(),
                    extras,
                },
            )
            .unwrap();
    }

    #[tokio::test]
    async fn discovery_registers_every_workspace_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        save_workspace(dir.path(), "acme");
        save_workspace(dir.path(), "personal");
        let app = SlackApp::new(dir.path().to_path_buf(), "cid".into(), "sec".into());
        let integrations: IntegrationRegistry = Default::default();
        let tools = ToolRegistry::new();

        let aliases = register_connected(&app, &integrations, &tools);
        assert_eq!(aliases, ["acme", "personal"]);
        let reg: Vec<_> = integrations.read().recover().values().cloned().collect();
        let mut names: Vec<_> = reg.iter().map(|i| i.name().to_string()).collect();
        names.sort();
        assert_eq!(names, ["slack@acme", "slack@personal"]);
        for i in &reg {
            assert!(i.is_connected().await);
            assert!(
                i.capabilities_summary()
                    .await
                    .unwrap()
                    .starts_with(i.name())
            );
        }
    }
}
