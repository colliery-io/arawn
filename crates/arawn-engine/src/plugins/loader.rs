//! Plugin discovery and loading — scans directories for plugin.json manifests.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use tracing::{info, warn};

use serde::{Deserialize, Serialize};

use super::manifest::PluginManifest;

/// Plugin identifier in `name@marketplace` format.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginIdentifier {
    pub name: String,
    pub marketplace: String,
}

impl PluginIdentifier {
    pub fn new(name: impl Into<String>, marketplace: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            marketplace: marketplace.into(),
        }
    }

    /// Parse from `name@marketplace` string.
    pub fn parse(s: &str) -> Option<Self> {
        let (name, marketplace) = s.split_once('@')?;
        if name.is_empty() || marketplace.is_empty() {
            return None;
        }
        Some(Self {
            name: name.to_string(),
            marketplace: marketplace.to_string(),
        })
    }

    /// For inline/session plugins loaded via --plugin-dir.
    pub fn inline(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            marketplace: "inline".into(),
        }
    }
}

impl std::fmt::Display for PluginIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.name, self.marketplace)
    }
}

/// Source of a loaded plugin.
#[derive(Debug, Clone, PartialEq)]
pub enum PluginSource {
    /// From the versioned cache (installed via marketplace).
    Cache,
    /// Loaded via --plugin-dir (session-only).
    Inline,
    /// Built-in plugin (registered in code).
    BuiltIn,
}

/// A discovered and validated plugin ready for component loading.
#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    /// Plugin identifier (name@marketplace).
    pub id: PluginIdentifier,
    /// The parsed manifest.
    pub manifest: PluginManifest,
    /// Absolute path to the plugin directory.
    pub plugin_dir: PathBuf,
    /// Where this plugin was discovered.
    pub source: PluginSource,
    /// Whether the plugin is enabled for the current project.
    pub enabled: bool,
    /// Resolved absolute paths for component directories.
    pub resolved_paths: ResolvedPaths,
}

/// Resolved absolute paths for plugin component directories.
#[derive(Debug, Clone, Default)]
pub struct ResolvedPaths {
    pub agents: Option<PathBuf>,
    pub skills: Option<PathBuf>,
    pub commands: Option<PathBuf>,
    pub tools: Option<PathBuf>,
    pub hooks_file: Option<PathBuf>,
}

impl LoadedPlugin {
    /// Plugin name (convenience accessor).
    pub fn name(&self) -> &str {
        &self.manifest.name
    }
}

/// Discover plugins from the versioned cache directory.
///
/// Scans `{plugins_root}/cache/{marketplace}/{plugin}/{version}/` for manifests.
/// For each marketplace/plugin, uses the latest (lexicographically highest) version.
pub fn discover_plugins(plugins_root: &Path) -> Vec<LoadedPlugin> {
    let cache_dir = plugins_root.join("cache");
    let mut plugins = HashMap::new();

    let marketplaces = match std::fs::read_dir(&cache_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    for market_entry in marketplaces.flatten() {
        let market_dir = market_entry.path();
        if !market_dir.is_dir() {
            continue;
        }
        let marketplace_name = market_entry.file_name().to_string_lossy().to_string();

        let plugin_dirs = match std::fs::read_dir(&market_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for plugin_entry in plugin_dirs.flatten() {
            let plugin_dir = plugin_entry.path();
            if !plugin_dir.is_dir() {
                continue;
            }
            let plugin_name = plugin_entry.file_name().to_string_lossy().to_string();

            // Find the latest version (lexicographic sort, last wins)
            let versions = match std::fs::read_dir(&plugin_dir) {
                Ok(e) => e,
                Err(_) => continue,
            };

            let mut latest_version: Option<(String, PathBuf)> = None;
            for ver_entry in versions.flatten() {
                let ver_dir = ver_entry.path();
                if !ver_dir.is_dir() {
                    continue;
                }
                let version = ver_entry.file_name().to_string_lossy().to_string();
                if latest_version.as_ref().is_none_or(|(v, _)| version > *v) {
                    latest_version = Some((version, ver_dir));
                }
            }

            if let Some((_version, ver_dir)) = latest_version
                && let Some(plugin) = load_plugin_from_dir(
                    &ver_dir,
                    &plugin_name,
                    &marketplace_name,
                    PluginSource::Cache,
                ) {
                    plugins.insert(plugin.id.to_string(), plugin);
                }
        }
    }

    plugins.into_values().collect()
}

/// Load a single plugin from a directory (for --plugin-dir flag).
///
/// Tags as `name@inline` since it's not from a marketplace.
pub fn load_plugin_dir(dir: &Path) -> Option<LoadedPlugin> {
    load_plugin_from_dir(dir, "", "inline", PluginSource::Inline).map(|mut p| {
        // For inline plugins, the name comes from the manifest
        p.id = PluginIdentifier::inline(&p.manifest.name);
        p
    })
}

/// Load a plugin from a directory, reading .claude-plugin/plugin.json or plugin.json.
fn load_plugin_from_dir(
    dir: &Path,
    default_name: &str,
    marketplace: &str,
    source: PluginSource,
) -> Option<LoadedPlugin> {
    match PluginManifest::from_dir(dir) {
        Ok(manifest) => {
            let errors = manifest.validate();
            if !errors.is_empty() {
                for error in &errors {
                    warn!(dir = ?dir, error = %error, "plugin manifest validation error");
                }
                return None;
            }

            let name = if manifest.name.is_empty() {
                default_name.to_string()
            } else {
                manifest.name.clone()
            };

            let id = PluginIdentifier::new(&name, marketplace);
            let resolved_paths = resolve_paths(&manifest, dir);

            info!(id = %id, dir = ?dir, "discovered plugin");

            Some(LoadedPlugin {
                id,
                manifest,
                plugin_dir: dir.to_path_buf(),
                source,
                enabled: true,
                resolved_paths,
            })
        }
        Err(e) => {
            warn!(dir = ?dir, error = %e, "failed to load plugin manifest");
            None
        }
    }
}

/// Resolve relative component paths against the plugin directory.
///
/// If a component path isn't declared in the manifest, auto-discovers by
/// convention: if `agents/`, `skills/`, `commands/`, `tools/` directories
/// exist next to the manifest, use them. This matches Claude Code's behavior
/// where plugins don't need to explicitly list every component.
fn resolve_paths(manifest: &PluginManifest, plugin_dir: &Path) -> ResolvedPaths {
    let resolve_or_discover = |declared: &Option<String>, convention: &str| -> Option<PathBuf> {
        if let Some(p) = declared {
            let stripped = p.strip_prefix("./").unwrap_or(p);
            return Some(plugin_dir.join(stripped));
        }
        // Auto-discover by convention
        let path = plugin_dir.join(convention);
        if path.is_dir() {
            Some(path)
        } else {
            None
        }
    };

    let hooks_file = match &manifest.hooks {
        Some(super::manifest::HooksField::Path(p)) => {
            let stripped = p.strip_prefix("./").unwrap_or(p);
            Some(plugin_dir.join(stripped))
        }
        _ => {
            // Auto-discover hooks/hooks.json
            let path = plugin_dir.join("hooks").join("hooks.json");
            if path.is_file() {
                Some(path)
            } else {
                None
            }
        }
    };

    ResolvedPaths {
        agents: resolve_or_discover(&manifest.agents, "agents"),
        skills: resolve_or_discover(&manifest.skills, "skills"),
        commands: resolve_or_discover(&manifest.commands, "commands"),
        tools: resolve_or_discover(&manifest.tools, "tools"),
        hooks_file,
    }
}

/// Registry of loaded plugins, queryable by identifier string.
pub struct PluginRegistry {
    plugins: RwLock<HashMap<String, LoadedPlugin>>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    /// Register a loaded plugin (keyed by id string: `name@marketplace`).
    pub fn register(&self, plugin: LoadedPlugin) {
        let key = plugin.id.to_string();
        self.plugins.write().unwrap().insert(key, plugin);
    }

    /// Get a plugin by identifier string (e.g. `metis@colliery-io-metis`).
    /// Also matches by name alone if unambiguous.
    pub fn get(&self, key: &str) -> Option<LoadedPlugin> {
        let plugins = self.plugins.read().unwrap();
        if let Some(p) = plugins.get(key) {
            return Some(p.clone());
        }
        // Fallback: match by name alone
        let matches: Vec<_> = plugins
            .values()
            .filter(|p| p.manifest.name == key)
            .collect();
        if matches.len() == 1 {
            Some(matches[0].clone())
        } else {
            None
        }
    }

    /// Get all registered plugins.
    pub fn all(&self) -> Vec<LoadedPlugin> {
        self.plugins.read().unwrap().values().cloned().collect()
    }

    /// Get only enabled plugins.
    pub fn enabled(&self) -> Vec<LoadedPlugin> {
        self.plugins
            .read()
            .unwrap()
            .values()
            .filter(|p| p.enabled)
            .cloned()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.plugins.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.read().unwrap().is_empty()
    }

    /// Set enable/disable state by identifier string.
    pub fn set_enabled(&self, key: &str, enabled: bool) {
        if let Some(plugin) = self.plugins.write().unwrap().get_mut(key) {
            plugin.enabled = enabled;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Create a cache-structured plugin: cache/{marketplace}/{plugin}/{version}/plugin.json
    fn write_cached_plugin(root: &Path, marketplace: &str, name: &str, version: &str, extra: &str) {
        let dir = root.join("cache").join(marketplace).join(name).join(version);
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = format!(r#"{{ "name": "{name}", "version": "{version}" {extra} }}"#);
        std::fs::write(dir.join("plugin.json"), manifest).unwrap();
    }

    /// Create a .claude-plugin/plugin.json style plugin.
    fn write_claude_plugin(root: &Path, marketplace: &str, name: &str, version: &str) {
        let dir = root.join("cache").join(marketplace).join(name).join(version);
        let claude_dir = dir.join(".claude-plugin");
        std::fs::create_dir_all(&claude_dir).unwrap();
        let manifest = format!(r#"{{ "name": "{name}", "version": "{version}" }}"#);
        std::fs::write(claude_dir.join("plugin.json"), manifest).unwrap();
    }

    #[test]
    fn discover_from_cache() {
        let root = TempDir::new().unwrap();
        write_cached_plugin(root.path(), "my-market", "plugin-a", "1.0.0", "");
        write_cached_plugin(root.path(), "my-market", "plugin-b", "2.0.0", "");

        let plugins = discover_plugins(root.path());
        assert_eq!(plugins.len(), 2);

        let ids: Vec<String> = plugins.iter().map(|p| p.id.to_string()).collect();
        assert!(ids.contains(&"plugin-a@my-market".to_string()));
        assert!(ids.contains(&"plugin-b@my-market".to_string()));
    }

    #[test]
    fn latest_version_wins() {
        let root = TempDir::new().unwrap();
        write_cached_plugin(root.path(), "market", "plugin", "1.0.0", "");
        write_cached_plugin(root.path(), "market", "plugin", "2.0.0", "");

        let plugins = discover_plugins(root.path());
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].manifest.version.as_deref(), Some("2.0.0"));
    }

    #[test]
    fn claude_plugin_path_discovered() {
        let root = TempDir::new().unwrap();
        write_claude_plugin(root.path(), "colliery-io-metis", "metis", "2.0.4");

        let plugins = discover_plugins(root.path());
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].id.to_string(), "metis@colliery-io-metis");
        assert_eq!(plugins[0].manifest.version.as_deref(), Some("2.0.4"));
    }

    #[test]
    fn missing_cache_dir_returns_empty() {
        let plugins = discover_plugins(Path::new("/nonexistent"));
        assert!(plugins.is_empty());
    }

    #[test]
    fn load_plugin_dir_inline() {
        let dir = TempDir::new().unwrap();
        let manifest = r#"{ "name": "my-local-plugin", "agents": "./agents" }"#;
        std::fs::write(dir.path().join("plugin.json"), manifest).unwrap();

        let plugin = load_plugin_dir(dir.path()).unwrap();
        assert_eq!(plugin.id.to_string(), "my-local-plugin@inline");
        assert_eq!(plugin.source, PluginSource::Inline);
    }

    #[test]
    fn identifier_parse_display() {
        let id = PluginIdentifier::parse("metis@colliery-io-metis").unwrap();
        assert_eq!(id.name, "metis");
        assert_eq!(id.marketplace, "colliery-io-metis");
        assert_eq!(id.to_string(), "metis@colliery-io-metis");
    }

    #[test]
    fn identifier_parse_invalid() {
        assert!(PluginIdentifier::parse("no-at-sign").is_none());
        assert!(PluginIdentifier::parse("@marketplace").is_none());
        assert!(PluginIdentifier::parse("name@").is_none());
    }

    #[test]
    fn registry_keyed_by_id() {
        let registry = PluginRegistry::new();

        let plugin = LoadedPlugin {
            id: PluginIdentifier::new("test", "market"),
            manifest: PluginManifest { name: "test".into(), ..Default::default() },
            plugin_dir: PathBuf::from("/tmp/test"),
            source: PluginSource::Cache,
            enabled: true,
            resolved_paths: ResolvedPaths::default(),
        };

        registry.register(plugin);
        assert!(registry.get("test@market").is_some());
        assert!(registry.get("test").is_some()); // name-only fallback
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn registry_enable_disable() {
        let registry = PluginRegistry::new();
        registry.register(LoadedPlugin {
            id: PluginIdentifier::new("test", "market"),
            manifest: PluginManifest { name: "test".into(), ..Default::default() },
            plugin_dir: PathBuf::from("/tmp/test"),
            source: PluginSource::Cache,
            enabled: true,
            resolved_paths: ResolvedPaths::default(),
        });

        assert_eq!(registry.enabled().len(), 1);
        registry.set_enabled("test@market", false);
        assert_eq!(registry.enabled().len(), 0);
        assert_eq!(registry.all().len(), 1);
    }
}
