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

/// Drop workspace `alias` and its tools. The other workspaces keep
/// theirs. Returns the names of the tools removed.
pub fn deregister(
    integrations: &IntegrationRegistry,
    tools: &ToolRegistry,
    alias: &str,
) -> Vec<String> {
    let removed: Vec<String> = arawn_integrations::slack::slack_tool_names(alias)
        .into_iter()
        .filter(|n| tools.unregister(n).is_some())
        .collect();
    let name = arawn_integrations::slack::workspace_name(alias);
    integrations.write().recover().remove(&name);
    info!(workspace = %name, tools = removed.len(), "Slack workspace removed");
    removed
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

    fn ctx() -> arawn_engine::EngineToolContext {
        let lens = arawn_core::Lens::scratch(std::env::temp_dir());
        arawn_engine::EngineToolContext::new(&lens, uuid::Uuid::new_v4())
    }

    #[tokio::test]
    async fn each_tool_calls_its_own_workspace_and_disconnect_removes_only_its_six() {
        let dir = tempfile::tempdir().unwrap();
        save_workspace(dir.path(), "acme");
        save_workspace(dir.path(), "personal");
        let app = SlackApp::new(dir.path().to_path_buf(), "cid".into(), "sec".into());
        let integrations: IntegrationRegistry = Default::default();
        let tools = ToolRegistry::new();
        register_connected(&app, &integrations, &tools);
        assert_eq!(tools.len(), 12);

        // A call goes to its own workspace: with the token gone from
        // acme only, acme's tool fails on acme and personal's does not
        // fail on a missing token.
        arawn_auth::TokenStore::open(dir.path())
            .unwrap()
            .delete("slack@acme")
            .unwrap();
        let params = serde_json::json!({ "channel": "C1", "name": "thumbsup", "timestamp": "1.2" });
        let err = tools
            .get("slack_acme_react")
            .unwrap()
            .execute(&ctx(), params.clone())
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("slack@acme"), "{err}");
        // personal's token has no reactions:write: the scope check, not
        // the token, stops it. No network either way.
        let err = tools
            .get("slack_personal_react")
            .unwrap()
            .execute(&ctx(), params)
            .await
            .unwrap_err()
            .to_string();
        assert!(
            !err.contains("slack@acme") && err.contains("reactions:write"),
            "{err}"
        );

        let removed = deregister(&integrations, &tools, "acme");
        assert_eq!(removed.len(), 6);
        assert_eq!(tools.len(), 6);
        assert!(
            arawn_integrations::slack::slack_tool_names("personal")
                .iter()
                .all(|n| tools.contains(n))
        );
        let names: Vec<String> = integrations.read().recover().keys().cloned().collect();
        assert_eq!(names, ["slack@personal"]);
    }

    #[tokio::test]
    async fn one_workspace_call_is_one_registry_lookup() {
        // NFR-002: a tool holds its workspace, so a call is the tool
        // registry's one map lookup and never reads the integration
        // registry: with that registry emptied, the call still reaches
        // acme's token (and stops at its scope check, before any network).
        let dir = tempfile::tempdir().unwrap();
        save_workspace(dir.path(), "acme");
        let app = SlackApp::new(dir.path().to_path_buf(), "cid".into(), "sec".into());
        let integrations: IntegrationRegistry = Default::default();
        let tools = ToolRegistry::new();
        register_connected(&app, &integrations, &tools);
        integrations.write().recover().clear();
        let params = serde_json::json!({ "channel": "C1", "name": "thumbsup", "timestamp": "1.2" });
        let err = tools
            .get("slack_acme_react")
            .unwrap()
            .execute(&ctx(), params)
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("reactions:write"), "{err}");
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
