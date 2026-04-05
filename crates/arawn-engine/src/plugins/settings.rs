//! Plugin enable/disable and user configuration settings.
//!
//! Reads enable/disable state and user config values from settings.json,
//! applies them to loaded plugins.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;
use tracing::warn;

use super::loader::LoadedPlugin;
use super::manifest::UserConfigField;

/// Plugin settings section from `.arawn/settings.json`.
///
/// ```json
/// {
///   "enabledPlugins": {
///     "metis@colliery-io-metis": true,
///     "unwanted@some-market": false
///   },
///   "pluginConfigs": {
///     "my-plugin": {
///       "options": { "API_KEY": "abc123" }
///     }
///   }
/// }
/// ```
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSettings {
    /// Map of `name@marketplace` → enabled (true/false).
    /// Plugins not listed are enabled by default.
    #[serde(default)]
    pub enabled_plugins: HashMap<String, bool>,
    /// Per-plugin user configuration values.
    #[serde(default)]
    pub plugin_configs: HashMap<String, PluginConfigEntry>,
}

/// Per-plugin user configuration entry.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PluginConfigEntry {
    /// User-provided option values.
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
}

/// Load plugin settings from a JSON settings file.
pub fn load_plugin_settings(path: &Path) -> PluginSettings {
    if !path.exists() {
        return PluginSettings::default();
    }

    match std::fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str::<PluginSettings>(&content) {
            Ok(settings) => settings,
            Err(e) => {
                warn!(path = %path.display(), error = %e, "failed to parse plugin settings");
                PluginSettings::default()
            }
        },
        Err(e) => {
            warn!(path = %path.display(), error = %e, "failed to read settings file");
            PluginSettings::default()
        }
    }
}

/// Apply enable/disable settings to a list of loaded plugins.
///
/// Looks up each plugin's `name@marketplace` identifier in the settings map.
/// If found, uses the boolean value. If not found, defaults to enabled.
pub fn apply_enable_disable(plugins: &mut [LoadedPlugin], settings: &PluginSettings) {
    for plugin in plugins.iter_mut() {
        let id_str = plugin.id.to_string();

        if let Some(&enabled) = settings.enabled_plugins.get(&id_str) {
            plugin.enabled = enabled;
        }
        // Also check by name alone (for backward compat / convenience)
        else if let Some(&enabled) = settings.enabled_plugins.get(&plugin.manifest.name) {
            plugin.enabled = enabled;
        }
        // Not listed → default enabled
    }
}

/// Validate user config values against the plugin manifest's `userConfig` declarations.
///
/// Returns a list of warning messages for missing required fields.
pub fn validate_user_config(
    plugin_name: &str,
    declarations: &HashMap<String, UserConfigField>,
    values: &HashMap<String, serde_json::Value>,
) -> Vec<String> {
    let mut warnings = Vec::new();

    for (key, field) in declarations {
        if field.required && !values.contains_key(key) {
            let msg = format!(
                "plugin '{}': missing required config field '{}' ({})",
                plugin_name,
                key,
                field.title.as_deref().unwrap_or(key)
            );
            warnings.push(msg);
        }
    }

    warnings
}

/// Get resolved user config values for a plugin, applying defaults.
pub fn resolve_user_config(
    declarations: &HashMap<String, UserConfigField>,
    values: &HashMap<String, serde_json::Value>,
) -> HashMap<String, serde_json::Value> {
    let mut resolved = HashMap::new();

    for (key, field) in declarations {
        if let Some(value) = values.get(key) {
            resolved.insert(key.clone(), value.clone());
        } else if let Some(ref default) = field.default {
            resolved.insert(key.clone(), default.clone());
        }
    }

    resolved
}

/// Convert resolved user config values to environment variables.
///
/// Keys are uppercased with a prefix: `PLUGIN_<KEY>`.
pub fn config_to_env_vars(
    config: &HashMap<String, serde_json::Value>,
) -> HashMap<String, String> {
    let mut env = HashMap::new();

    for (key, value) in config {
        let env_key = format!("PLUGIN_{}", key.to_uppercase());
        let env_value = match value {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        env.insert(env_key, env_value);
    }

    env
}

/// Substitute `${user_config.KEY}` placeholders in a string with resolved values.
pub fn substitute_user_config(template: &str, config: &HashMap<String, serde_json::Value>) -> String {
    let mut result = template.to_string();
    for (key, value) in config {
        let placeholder = format!("${{user_config.{}}}", key);
        let replacement = match value {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        result = result.replace(&placeholder, &replacement);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::loader::{LoadedPlugin, PluginIdentifier, PluginSource, ResolvedPaths};
    use crate::plugins::manifest::PluginManifest;
    use std::path::PathBuf;

    fn make_plugin(name: &str, marketplace: &str) -> LoadedPlugin {
        LoadedPlugin {
            id: PluginIdentifier::new(name, marketplace),
            manifest: PluginManifest {
                name: name.into(),
                ..Default::default()
            },
            plugin_dir: PathBuf::from("/tmp"),
            source: PluginSource::Cache,
            enabled: true,
            resolved_paths: ResolvedPaths::default(),
        }
    }

    #[test]
    fn default_all_enabled() {
        let mut plugins = vec![make_plugin("a", "market"), make_plugin("b", "market")];
        let settings = PluginSettings::default();

        apply_enable_disable(&mut plugins, &settings);
        assert!(plugins[0].enabled);
        assert!(plugins[1].enabled);
    }

    #[test]
    fn disable_by_id() {
        let mut plugins = vec![
            make_plugin("a", "market"),
            make_plugin("b", "market"),
            make_plugin("c", "market"),
        ];
        let mut enabled = HashMap::new();
        enabled.insert("b@market".into(), false);
        let settings = PluginSettings {
            enabled_plugins: enabled,
            ..Default::default()
        };

        apply_enable_disable(&mut plugins, &settings);
        assert!(plugins[0].enabled);
        assert!(!plugins[1].enabled);
        assert!(plugins[2].enabled);
    }

    #[test]
    fn disable_by_name_fallback() {
        let mut plugins = vec![make_plugin("a", "market")];
        let mut enabled = HashMap::new();
        enabled.insert("a".into(), false); // name only, no @marketplace
        let settings = PluginSettings {
            enabled_plugins: enabled,
            ..Default::default()
        };

        apply_enable_disable(&mut plugins, &settings);
        assert!(!plugins[0].enabled);
    }

    #[test]
    fn validate_missing_required() {
        let mut declarations = HashMap::new();
        declarations.insert(
            "API_KEY".into(),
            UserConfigField {
                field_type: "string".into(),
                title: Some("API Key".into()),
                description: None,
                required: true,
                default: None,
            },
        );
        declarations.insert(
            "OPTIONAL".into(),
            UserConfigField {
                field_type: "string".into(),
                title: None,
                description: None,
                required: false,
                default: None,
            },
        );

        let values = HashMap::new(); // no values provided
        let warnings = validate_user_config("test-plugin", &declarations, &values);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("API_KEY"));
        assert!(warnings[0].contains("test-plugin"));
    }

    #[test]
    fn validate_all_present() {
        let mut declarations = HashMap::new();
        declarations.insert(
            "API_KEY".into(),
            UserConfigField {
                field_type: "string".into(),
                title: None,
                description: None,
                required: true,
                default: None,
            },
        );

        let mut values = HashMap::new();
        values.insert("API_KEY".into(), serde_json::json!("abc123"));

        let warnings = validate_user_config("test", &declarations, &values);
        assert!(warnings.is_empty());
    }

    #[test]
    fn resolve_with_defaults() {
        let mut declarations = HashMap::new();
        declarations.insert(
            "PORT".into(),
            UserConfigField {
                field_type: "number".into(),
                title: None,
                description: None,
                required: false,
                default: Some(serde_json::json!(8080)),
            },
        );
        declarations.insert(
            "HOST".into(),
            UserConfigField {
                field_type: "string".into(),
                title: None,
                description: None,
                required: false,
                default: None,
            },
        );

        let values = HashMap::new();
        let resolved = resolve_user_config(&declarations, &values);

        assert_eq!(resolved.get("PORT"), Some(&serde_json::json!(8080)));
        assert!(resolved.get("HOST").is_none()); // no default, no value
    }

    #[test]
    fn resolve_value_overrides_default() {
        let mut declarations = HashMap::new();
        declarations.insert(
            "PORT".into(),
            UserConfigField {
                field_type: "number".into(),
                title: None,
                description: None,
                required: false,
                default: Some(serde_json::json!(8080)),
            },
        );

        let mut values = HashMap::new();
        values.insert("PORT".into(), serde_json::json!(3000));

        let resolved = resolve_user_config(&declarations, &values);
        assert_eq!(resolved.get("PORT"), Some(&serde_json::json!(3000)));
    }

    #[test]
    fn config_to_env() {
        let mut config = HashMap::new();
        config.insert("API_KEY".into(), serde_json::json!("abc123"));
        config.insert("PORT".into(), serde_json::json!(8080));

        let env = config_to_env_vars(&config);
        assert_eq!(env.get("PLUGIN_API_KEY"), Some(&"abc123".to_string()));
        assert_eq!(env.get("PLUGIN_PORT"), Some(&"8080".to_string()));
    }

    #[test]
    fn substitute_placeholders() {
        let mut config = HashMap::new();
        config.insert("API_KEY".into(), serde_json::json!("secret123"));
        config.insert("PORT".into(), serde_json::json!(8080));

        let template = "curl -H 'Auth: ${user_config.API_KEY}' http://localhost:${user_config.PORT}";
        let result = substitute_user_config(template, &config);
        assert_eq!(result, "curl -H 'Auth: secret123' http://localhost:8080");
    }

    #[test]
    fn substitute_no_match_left_alone() {
        let config = HashMap::new();
        let result = substitute_user_config("no ${user_config.MISSING} here", &config);
        assert_eq!(result, "no ${user_config.MISSING} here");
    }

    #[test]
    fn load_settings_from_json() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("settings.json");

        std::fs::write(
            &path,
            r#"{
                "enabledPlugins": {
                    "metis@colliery-io-metis": true,
                    "unwanted@market": false
                },
                "pluginConfigs": {
                    "my-plugin": {
                        "options": { "API_KEY": "test123" }
                    }
                }
            }"#,
        )
        .unwrap();

        let settings = load_plugin_settings(&path);
        assert_eq!(settings.enabled_plugins.get("metis@colliery-io-metis"), Some(&true));
        assert_eq!(settings.enabled_plugins.get("unwanted@market"), Some(&false));

        let config = settings.plugin_configs.get("my-plugin").unwrap();
        assert_eq!(
            config.options.get("API_KEY"),
            Some(&serde_json::json!("test123"))
        );
    }

    #[test]
    fn load_missing_settings_returns_defaults() {
        let settings = load_plugin_settings(Path::new("/nonexistent/settings.json"));
        assert!(settings.enabled_plugins.is_empty());
        assert!(settings.plugin_configs.is_empty());
    }
}
