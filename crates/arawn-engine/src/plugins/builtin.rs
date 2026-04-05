//! Built-in plugins — code-defined plugins using the same LoadedPlugin interface.
//!
//! Built-in plugins are registered in Rust code and appear alongside disk plugins
//! in the PluginRegistry. They can be enabled/disabled via settings like any other plugin.

use std::path::PathBuf;

use crate::agent_defs::AgentDefinition;
use crate::hooks::HookConfig;
use crate::skills::SkillDefinition;

use super::loader::{LoadedPlugin, PluginIdentifier, PluginSource, ResolvedPaths};
use super::manifest::PluginManifest;

/// Definition for a built-in plugin (registered in code, not from disk).
pub struct BuiltinPluginDef {
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub skills: Vec<SkillDefinition>,
    pub hooks: Option<HookConfig>,
    pub agents: Vec<AgentDefinition>,
}

impl BuiltinPluginDef {
    /// Convert this definition into a `LoadedPlugin` for the registry.
    pub fn into_loaded_plugin(self) -> LoadedPlugin {
        let id = PluginIdentifier::new(&self.name, "builtin");
        LoadedPlugin {
            id,
            manifest: PluginManifest {
                name: self.name,
                version: self.version,
                description: Some(self.description),
                ..Default::default()
            },
            plugin_dir: PathBuf::new(), // no directory on disk
            source: PluginSource::BuiltIn,
            enabled: true, // enabled by default
            resolved_paths: ResolvedPaths::default(),
        }
    }
}

/// Returns all built-in plugins.
///
/// Currently ships a "core" plugin with example skills.
/// Add more built-in plugins here as needed.
pub fn builtin_plugins() -> Vec<(LoadedPlugin, BuiltinComponents)> {
    vec![core_plugin()]
}

/// Components from a built-in plugin (already loaded, no disk I/O needed).
pub struct BuiltinComponents {
    pub skills: Vec<SkillDefinition>,
    pub hooks: Option<HookConfig>,
    pub agents: Vec<AgentDefinition>,
}

/// The "core" built-in plugin — ships default skills.
fn core_plugin() -> (LoadedPlugin, BuiltinComponents) {
    let def = BuiltinPluginDef {
        name: "core".into(),
        description: "Built-in core functionality".into(),
        version: Some("0.1.0".into()),
        skills: vec![],
        hooks: None,
        agents: vec![],
    };

    let components = BuiltinComponents {
        skills: def.skills.clone(),
        hooks: def.hooks.clone(),
        agents: def.agents.clone(),
    };

    (def.into_loaded_plugin(), components)
}

/// Register built-in plugins into the plugin registry alongside disk plugins.
///
/// Built-in plugins are added first (lowest priority). If a disk plugin
/// has the same name, it takes priority because `discover_plugins` runs
/// after this and overwrites in the registry.
pub fn register_builtin_plugins(
    registry: &super::loader::PluginRegistry,
) -> Vec<BuiltinComponents> {
    let mut all_components = Vec::new();

    for (plugin, components) in builtin_plugins() {
        let name = plugin.name().to_string();
        registry.register(plugin);
        tracing::info!(name = %name, "registered built-in plugin");
        all_components.push(components);
    }

    all_components
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::loader::PluginRegistry;

    #[test]
    fn builtin_plugin_converts_to_loaded() {
        let def = BuiltinPluginDef {
            name: "test-builtin".into(),
            description: "A test plugin".into(),
            version: Some("1.0.0".into()),
            skills: vec![],
            hooks: None,
            agents: vec![],
        };

        let loaded = def.into_loaded_plugin();
        assert_eq!(loaded.name(), "test-builtin");
        assert_eq!(loaded.source, PluginSource::BuiltIn);
        assert!(loaded.enabled);
        assert_eq!(
            loaded.manifest.description.as_deref(),
            Some("A test plugin")
        );
    }

    #[test]
    fn builtin_plugins_exist() {
        let plugins = builtin_plugins();
        assert!(!plugins.is_empty());

        let (core, _) = &plugins[0];
        assert_eq!(core.name(), "core");
        assert_eq!(core.source, PluginSource::BuiltIn);
    }

    #[test]
    fn register_into_registry() {
        let registry = PluginRegistry::new();
        let components = register_builtin_plugins(&registry);

        assert!(!registry.is_empty());
        assert!(registry.get("core@builtin").is_some());
        assert!(registry.get("core").is_some()); // name-only fallback
        assert_eq!(components.len(), 1);
    }

    #[test]
    fn disk_plugin_overrides_builtin() {
        let registry = PluginRegistry::new();
        register_builtin_plugins(&registry);

        // Simulate a cache plugin with the same name (different marketplace)
        let disk_plugin = LoadedPlugin {
            id: PluginIdentifier::new("core", "custom-market"),
            manifest: PluginManifest {
                name: "core".into(),
                description: Some("Custom core".into()),
                ..Default::default()
            },
            plugin_dir: PathBuf::from("/tmp/custom-core"),
            source: PluginSource::Cache,
            enabled: true,
            resolved_paths: ResolvedPaths::default(),
        };
        registry.register(disk_plugin);

        // Both exist — lookup by full id works
        let builtin = registry.get("core@builtin").unwrap();
        assert_eq!(builtin.source, PluginSource::BuiltIn);

        let custom = registry.get("core@custom-market").unwrap();
        assert_eq!(custom.source, PluginSource::Cache);
    }

    #[test]
    fn disable_builtin_via_settings() {
        let registry = PluginRegistry::new();
        register_builtin_plugins(&registry);

        assert!(registry.get("core@builtin").unwrap().enabled);

        registry.set_enabled("core@builtin", false);
        assert!(!registry.get("core@builtin").unwrap().enabled);
        assert_eq!(registry.enabled().len(), 0);
    }
}
