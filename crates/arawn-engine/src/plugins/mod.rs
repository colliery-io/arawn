//! Plugin framework — unified plugin manifest for tools, agents, skills, hooks, and MCP servers.
//!
//! Plugins are directories with a `plugin.json` manifest that declares what
//! components the plugin provides. The plugin loader discovers, validates, and
//! loads plugins from `~/.arawn/plugins/` and `.arawn/plugins/`.

mod builtin;
mod components;
mod installer;
mod loader;
mod manifest;
mod marketplace;
mod runtime;
mod settings;

pub use builtin::{
    BuiltinComponents, BuiltinPluginDef, builtin_plugins, register_builtin_plugins,
};
pub use installer::{
    InstallRecord, InstallScope, InstalledPluginsRegistry, install_plugin, uninstall_plugin,
};
pub use components::{
    PluginComponents, load_plugin_components, merge_plugin_hooks, register_plugin_skills,
};
pub use loader::{
    LoadedPlugin, PluginIdentifier, PluginRegistry, PluginSource, ResolvedPaths,
    discover_plugins, load_plugin_dir,
};
pub use marketplace::{
    KnownMarketplaces, MarketplaceEntry, MarketplaceManifest, MarketplacePlugin,
    MarketplaceSource, PluginSourceRef, add_marketplace, fetch_marketplace, list_marketplaces,
    resolve_plugin,
};
pub use manifest::{
    HooksField, McpServerDef, PluginAuthor, PluginError, PluginManifest, UserConfigField,
};
pub use runtime::{PluginLoadResult, PluginMcpServer, PluginRuntime};
pub use settings::{
    PluginSettings, apply_enable_disable, config_to_env_vars, load_plugin_settings,
    resolve_user_config, substitute_user_config, validate_user_config,
};
