//! Plugin component loading — loads agents, skills, hooks, MCP configs, and tools
//! from a plugin's declared directories into the engine's registries.

use tracing::{info, warn};

use crate::agent_defs::{AgentDefinition, AgentSource, load_agents_dir};
use crate::hooks::{HookConfig, load_hooks_from_file};
use crate::skills::{SkillDefinition, SkillSource, SkillRegistry, load_skills_dir};

use super::loader::LoadedPlugin;
use super::manifest::HooksField;

/// Result of loading components from a single plugin.
#[derive(Debug, Default)]
pub struct PluginComponents {
    /// Agent definitions loaded from the plugin.
    pub agents: Vec<AgentDefinition>,
    /// Skill definitions loaded from the plugin.
    pub skills: Vec<SkillDefinition>,
    /// Hook configuration loaded from the plugin.
    pub hooks: Option<HookConfig>,
    /// MCP server names declared by the plugin (configs stored in manifest).
    pub mcp_server_names: Vec<String>,
    /// Errors encountered during loading (non-fatal — other components still load).
    pub errors: Vec<String>,
}

/// Load all components from a plugin into a `PluginComponents` struct.
///
/// Individual component loading errors are collected but don't prevent
/// other components from loading.
pub fn load_plugin_components(plugin: &LoadedPlugin) -> PluginComponents {
    let mut result = PluginComponents::default();
    let plugin_name = plugin.name();

    // --- Agents ---
    if let Some(ref agents_dir) = plugin.resolved_paths.agents {
        if agents_dir.exists() {
            let mut agents = load_agents_dir(agents_dir);
            // Namespace agent names with plugin name
            for agent in &mut agents {
                agent.name = format!("{}:{}", plugin_name, agent.name);
                agent.source = AgentSource::User; // Plugin agents treated as user-defined
            }
            info!(plugin = plugin_name, count = agents.len(), "loaded plugin agents");
            result.agents = agents;
        } else {
            result.errors.push(format!(
                "agents directory does not exist: {}",
                agents_dir.display()
            ));
        }
    }

    // --- Skills ---
    if let Some(ref skills_dir) = plugin.resolved_paths.skills {
        if skills_dir.exists() {
            let mut skills = load_skills_dir(skills_dir, SkillSource::Plugin(plugin_name.into()));
            // Namespace skill names with plugin name
            for skill in &mut skills {
                skill.name = format!("{}:{}", plugin_name, skill.name);
            }
            info!(plugin = plugin_name, count = skills.len(), "loaded plugin skills");
            result.skills = skills;
        } else {
            result.errors.push(format!(
                "skills directory does not exist: {}",
                skills_dir.display()
            ));
        }
    }

    // --- Hooks ---
    match &plugin.manifest.hooks {
        Some(HooksField::Path(_)) => {
            if let Some(ref hooks_path) = plugin.resolved_paths.hooks_file {
                if hooks_path.exists() {
                    let config = load_hooks_from_file(hooks_path);
                    if !config.is_empty() {
                        info!(
                            plugin = plugin_name,
                            hook_config = %serde_json::to_string(&config).unwrap_or_default(),
                            "loaded plugin hooks — these run UNSANDBOXED shell commands"
                        );
                        result.hooks = Some(config);
                    }
                } else {
                    result.errors.push(format!(
                        "hooks file does not exist: {}",
                        hooks_path.display()
                    ));
                }
            }
        }
        Some(HooksField::Inline(config)) => {
            if !config.is_empty() {
                info!(
                    plugin = plugin_name,
                    hook_config = %serde_json::to_string(config).unwrap_or_default(),
                    "loaded inline plugin hooks — these run UNSANDBOXED shell commands"
                );
                result.hooks = Some(config.clone());
            }
        }
        None => {}
    }

    // --- MCP Servers ---
    if let Some(ref servers) = plugin.manifest.mcp_servers {
        let names: Vec<String> = servers.keys().cloned().collect();
        if !names.is_empty() {
            info!(plugin = plugin_name, servers = ?names, "found plugin MCP servers");
            result.mcp_server_names = names;
        }
    }

    // Log any errors
    for error in &result.errors {
        warn!(plugin = plugin_name, error, "plugin component loading error");
    }

    result
}

/// Register a plugin's skills into a SkillRegistry.
pub fn register_plugin_skills(registry: &SkillRegistry, skills: Vec<SkillDefinition>) {
    for skill in skills {
        registry.register(skill);
    }
}

/// Merge a plugin's hooks into an existing HookConfig.
pub fn merge_plugin_hooks(target: &mut HookConfig, plugin_hooks: HookConfig) {
    target.merge(plugin_hooks);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::loader::{PluginIdentifier, PluginSource, ResolvedPaths};
    use crate::plugins::manifest::PluginManifest;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn make_plugin(dir: &TempDir, name: &str, paths: ResolvedPaths) -> LoadedPlugin {
        LoadedPlugin {
            id: PluginIdentifier::new(name, "test-market"),
            manifest: PluginManifest {
                name: name.into(),
                ..Default::default()
            },
            plugin_dir: dir.path().to_path_buf(),
            source: PluginSource::Cache,
            enabled: true,
            resolved_paths: paths,
        }
    }

    #[test]
    fn load_agents_from_plugin() {
        let dir = TempDir::new().unwrap();
        let agents_dir = dir.path().join("agents");
        std::fs::create_dir(&agents_dir).unwrap();
        std::fs::write(
            agents_dir.join("reviewer.md"),
            r#"---
name: reviewer
description: Reviews code
---

Review the code carefully.
"#,
        )
        .unwrap();

        let plugin = make_plugin(
            &dir,
            "my-plugin",
            ResolvedPaths {
                agents: Some(agents_dir),
                ..Default::default()
            },
        );

        let components = load_plugin_components(&plugin);
        assert_eq!(components.agents.len(), 1);
        assert_eq!(components.agents[0].name, "my-plugin:reviewer");
        assert!(components.errors.is_empty());
    }

    #[test]
    fn load_skills_from_plugin() {
        let dir = TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();
        std::fs::write(
            skills_dir.join("deploy.md"),
            r#"---
description: Deploy the app
---

Deploy it.
"#,
        )
        .unwrap();

        let plugin = make_plugin(
            &dir,
            "my-plugin",
            ResolvedPaths {
                skills: Some(skills_dir),
                ..Default::default()
            },
        );

        let components = load_plugin_components(&plugin);
        assert_eq!(components.skills.len(), 1);
        assert_eq!(components.skills[0].name, "my-plugin:deploy");
        assert!(matches!(
            components.skills[0].source,
            SkillSource::Plugin(ref name) if name == "my-plugin"
        ));
    }

    #[test]
    fn load_hooks_from_file_path() {
        let dir = TempDir::new().unwrap();
        let hooks_path = dir.path().join("hooks.json");

        // Write valid hooks JSON
        std::fs::write(
            &hooks_path,
            r#"{
                "hooks": {
                    "PreToolUse": [{
                        "matcher": "Bash",
                        "hooks": [{ "type": "command", "command": "exit 0" }]
                    }]
                }
            }"#,
        )
        .unwrap();

        let mut manifest = PluginManifest {
            name: "test".into(),
            ..Default::default()
        };
        manifest.hooks = Some(HooksField::Path("./hooks.json".into()));

        let plugin = LoadedPlugin {
            id: PluginIdentifier::new("test", "test-market"),
            manifest,
            plugin_dir: dir.path().to_path_buf(),
            source: PluginSource::Cache,
            enabled: true,
            resolved_paths: ResolvedPaths {
                hooks_file: Some(hooks_path),
                ..Default::default()
            },
        };

        let components = load_plugin_components(&plugin);
        assert!(components.hooks.is_some());
    }

    #[test]
    fn load_inline_hooks() {
        let dir = TempDir::new().unwrap();

        let hook_config: HookConfig = serde_json::from_value(serde_json::json!({
            "SessionStart": [{
                "hooks": [{ "type": "command", "command": "echo hello" }]
            }]
        }))
        .unwrap();

        let mut manifest = PluginManifest {
            name: "test".into(),
            ..Default::default()
        };
        manifest.hooks = Some(HooksField::Inline(hook_config));

        let plugin = LoadedPlugin {
            id: PluginIdentifier::new("test", "test-market"),
            manifest,
            plugin_dir: dir.path().to_path_buf(),
            source: PluginSource::Cache,
            enabled: true,
            resolved_paths: ResolvedPaths::default(),
        };

        let components = load_plugin_components(&plugin);
        assert!(components.hooks.is_some());
    }

    #[test]
    fn mcp_servers_extracted() {
        let dir = TempDir::new().unwrap();

        let manifest: PluginManifest = serde_json::from_value(serde_json::json!({
            "name": "test",
            "mcpServers": {
                "sqlite": { "command": "uvx", "args": ["mcp-server-sqlite"] },
                "github": { "command": "npx", "args": ["-y", "@mcp/github"] }
            }
        }))
        .unwrap();

        let plugin = LoadedPlugin {
            id: PluginIdentifier::new("test", "test-market"),
            manifest,
            plugin_dir: dir.path().to_path_buf(),
            source: PluginSource::Cache,
            enabled: true,
            resolved_paths: ResolvedPaths::default(),
        };

        let components = load_plugin_components(&plugin);
        assert_eq!(components.mcp_server_names.len(), 2);
        assert!(components.mcp_server_names.contains(&"sqlite".to_string()));
        assert!(components.mcp_server_names.contains(&"github".to_string()));
    }

    #[test]
    fn missing_dir_produces_error_not_panic() {
        let dir = TempDir::new().unwrap();
        let plugin = make_plugin(
            &dir,
            "my-plugin",
            ResolvedPaths {
                agents: Some(PathBuf::from("/nonexistent/agents")),
                skills: Some(PathBuf::from("/nonexistent/skills")),
                ..Default::default()
            },
        );

        let components = load_plugin_components(&plugin);
        assert!(components.agents.is_empty());
        assert!(components.skills.is_empty());
        assert_eq!(components.errors.len(), 2);
    }

    #[test]
    fn empty_plugin_loads_nothing() {
        let dir = TempDir::new().unwrap();
        let plugin = make_plugin(&dir, "empty", ResolvedPaths::default());

        let components = load_plugin_components(&plugin);
        assert!(components.agents.is_empty());
        assert!(components.skills.is_empty());
        assert!(components.hooks.is_none());
        assert!(components.mcp_server_names.is_empty());
        assert!(components.errors.is_empty());
    }

    #[test]
    fn register_skills_into_registry() {
        let registry = SkillRegistry::new();
        let skills = vec![SkillDefinition {
            name: "test-plugin:deploy".into(),
            description: "Deploy".into(),
            prompt: "Do it".into(),
            argument_hint: None,
            allowed_tools: None,
            model: None,
            user_invocable: true,
            source: SkillSource::Plugin("test-plugin".into()),
        }];

        register_plugin_skills(&registry, skills);
        assert!(registry.get("test-plugin:deploy").is_some());
    }

    #[test]
    fn merge_hooks_into_config() {
        let mut target = HookConfig::default();
        let plugin_hooks: HookConfig = serde_json::from_value(serde_json::json!({
            "PreToolUse": [{
                "matcher": "Bash",
                "hooks": [{ "type": "command", "command": "exit 0" }]
            }]
        }))
        .unwrap();

        merge_plugin_hooks(&mut target, plugin_hooks);
        assert!(!target.is_empty());
    }
}
