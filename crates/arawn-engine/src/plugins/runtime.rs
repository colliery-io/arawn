//! Plugin runtime — startup loading and hot-reload for long-lived processes.
//!
//! Call `PluginRuntime::new()` at startup, then `.load_all()` to discover and
//! register all plugin components. For long-lived processes, call `.watch()`
//! to hot-reload when plugins are installed or changed.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::agent_defs::AgentDefinition;
use crate::hooks::HookConfig;
use crate::skills::{SkillDefinition, SkillRegistry};

use super::builtin::register_builtin_plugins;
use super::components::load_plugin_components;
use super::loader::{PluginRegistry, discover_plugins, load_plugin_dir};
use super::settings::{apply_enable_disable, load_plugin_settings};

/// An MCP server config extracted from a plugin manifest, ready for connection.
#[derive(Debug, Clone)]
pub struct PluginMcpServer {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
    pub plugin_id: String,
}

/// Result of loading all plugins — the components ready to wire into the engine.
pub struct PluginLoadResult {
    pub agents: Vec<AgentDefinition>,
    pub skills: Vec<SkillDefinition>,
    pub hooks: HookConfig,
    pub mcp_servers: Vec<PluginMcpServer>,
}

/// Plugin runtime — manages plugin lifecycle for a running arawn instance.
pub struct PluginRuntime {
    /// Root directory for plugins (~/.arawn/plugins/).
    plugins_root: PathBuf,
    /// Project settings path (.arawn/settings.json) for enable/disable.
    settings_path: Option<PathBuf>,
    /// Extra directories to load as inline plugins (--plugin-dir).
    plugin_dirs: Vec<PathBuf>,
    /// The plugin registry tracking all loaded plugins.
    pub registry: Arc<PluginRegistry>,
}

impl PluginRuntime {
    pub fn new(plugins_root: PathBuf) -> Self {
        Self {
            plugins_root,
            settings_path: None,
            plugin_dirs: Vec::new(),
            registry: Arc::new(PluginRegistry::new()),
        }
    }

    pub fn with_settings(mut self, path: PathBuf) -> Self {
        self.settings_path = Some(path);
        self
    }

    pub fn with_plugin_dir(mut self, dir: PathBuf) -> Self {
        self.plugin_dirs.push(dir);
        self
    }

    /// Discover, load, and register all plugins. Returns components to wire into the engine.
    pub fn load_all(&self, skill_registry: &Arc<SkillRegistry>) -> PluginLoadResult {
        let mut result = PluginLoadResult {
            agents: Vec::new(),
            skills: Vec::new(),
            hooks: HookConfig::default(),
            mcp_servers: Vec::new(),
        };

        // 1. Register built-in plugins
        let builtin_components = register_builtin_plugins(&self.registry);
        for bc in builtin_components {
            result.agents.extend(bc.agents);
            result.skills.extend(bc.skills);
            if let Some(hooks) = bc.hooks {
                result.hooks.merge(hooks);
            }
        }

        // 2. Discover plugins from cache
        let mut plugins = discover_plugins(&self.plugins_root);

        // 3. Load inline plugins (--plugin-dir)
        for dir in &self.plugin_dirs {
            if let Some(plugin) = load_plugin_dir(dir) {
                plugins.push(plugin);
            }
        }

        // 4. Apply enable/disable from settings
        if let Some(ref settings_path) = self.settings_path {
            let settings = load_plugin_settings(settings_path);
            apply_enable_disable(&mut plugins, &settings);
        }

        // 5. Load components from each enabled plugin
        let enabled_count = plugins.iter().filter(|p| p.enabled).count();
        let total_count = plugins.len();

        for plugin in &plugins {
            if !plugin.enabled {
                info!(plugin = %plugin.id, "plugin disabled, skipping");
                continue;
            }

            let components = load_plugin_components(plugin);

            // Register agents
            result.agents.extend(components.agents);

            // Register skills
            for skill in components.skills {
                skill_registry.register(skill.clone());
                result.skills.push(skill);
            }

            // Merge hooks
            if let Some(hooks) = components.hooks {
                result.hooks.merge(hooks);
            }

            // Extract MCP server configs from manifest
            if let Some(ref servers) = plugin.manifest.mcp_servers {
                for (name, def) in servers {
                    result.mcp_servers.push(PluginMcpServer {
                        name: name.clone(),
                        command: def.command.clone(),
                        args: def.args.clone(),
                        env: def.env.clone(),
                        plugin_id: plugin.id.to_string(),
                    });
                }
            }

            // Register into plugin registry
            self.registry.register(plugin.clone());
        }

        info!(
            total = total_count,
            enabled = enabled_count,
            agents = result.agents.len(),
            skills = result.skills.len(),
            mcp_servers = result.mcp_servers.len(),
            "plugins loaded"
        );

        result
    }

    /// Spawn a file watcher that hot-reloads plugins when the cache directory changes.
    ///
    /// For long-lived processes (serve mode). Re-discovers and re-loads all plugins
    /// when files in the cache directory are created, modified, or removed.
    ///
    /// Optional `notify` callback is invoked after each reload completes (success
    /// or failure) with a one-line human-readable status message + a "is_error"
    /// flag. Wired by the binary to a server-wide broadcast so the TUI can
    /// surface the reload outcome.
    pub fn watch(
        &self,
        skill_registry: Arc<SkillRegistry>,
        notify: Option<Arc<dyn Fn(bool, String) + Send + Sync>>,
    ) -> tokio::task::JoinHandle<()> {
        let plugins_root = self.plugins_root.clone();
        let settings_path = self.settings_path.clone();
        let plugin_dirs = self.plugin_dirs.clone();
        let plugin_registry = Arc::clone(&self.registry);

        tokio::spawn(async move {
            let cache_dir = plugins_root.join("cache");
            if let Err(e) = std::fs::create_dir_all(&cache_dir) {
                warn!(error = %e, "failed to create plugins cache dir");
                return;
            }

            let (tx, mut rx) = mpsc::channel::<notify::Result<notify::Event>>(64);

            let mut watcher = match RecommendedWatcher::new(
                move |res| {
                    let _ = tx.blocking_send(res);
                },
                notify::Config::default().with_poll_interval(Duration::from_secs(2)),
            ) {
                Ok(w) => w,
                Err(e) => {
                    warn!(error = %e, "failed to create plugin watcher");
                    return;
                }
            };

            if let Err(e) = watcher.watch(&cache_dir, RecursiveMode::Recursive) {
                warn!(error = %e, "failed to watch plugins cache dir");
                return;
            }

            info!(dir = ?cache_dir, "plugin hot-reload watcher started");

            loop {
                let event = match rx.recv().await {
                    Some(Ok(event)) => event,
                    Some(Err(e)) => {
                        warn!(error = %e, "plugin watcher error");
                        continue;
                    }
                    None => break,
                };

                if !matches!(
                    event.kind,
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                ) {
                    continue;
                }

                // Debounce: drain events for 1s
                let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
                loop {
                    match tokio::time::timeout_at(deadline, rx.recv()).await {
                        Ok(Some(Ok(_))) => continue,
                        Ok(Some(Err(_))) => continue,
                        _ => break,
                    }
                }

                info!("plugin cache changed, hot-reloading");

                // Re-discover all plugins
                let mut plugins = discover_plugins(&plugins_root);
                for dir in &plugin_dirs {
                    if let Some(plugin) = load_plugin_dir(dir) {
                        plugins.push(plugin);
                    }
                }

                if let Some(ref settings_path) = settings_path {
                    let settings = load_plugin_settings(settings_path);
                    apply_enable_disable(&mut plugins, &settings);
                }

                // Re-load components from each enabled plugin
                let mut total_skills = 0;
                let mut total_agents = 0;
                for plugin in &plugins {
                    if !plugin.enabled {
                        continue;
                    }

                    let components = load_plugin_components(plugin);

                    for skill in components.skills {
                        skill_registry.register(skill);
                        total_skills += 1;
                    }

                    total_agents += components.agents.len();
                    // Note: agents loaded here are logged but not injected into
                    // existing AgentTool instances. New sessions will pick them up
                    // when the engine rebuilds the system prompt (which reads skills
                    // from the registry dynamically each turn).

                    plugin_registry.register(plugin.clone());
                }

                info!(
                    plugins = plugins.len(),
                    skills = total_skills,
                    agents = total_agents,
                    "hot-reload complete"
                );

                if let Some(ref n) = notify {
                    n(
                        false,
                        format!(
                            "plugins reloaded: {} plugin(s), {} skill(s), {} agent(s)",
                            plugins.len(),
                            total_skills,
                            total_agents
                        ),
                    );
                }
            }
        })
    }
}
