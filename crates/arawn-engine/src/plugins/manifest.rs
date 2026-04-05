//! Plugin manifest — deserialization and validation of plugin.json.

use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};

use crate::hooks::HookConfig;

/// A plugin manifest loaded from `plugin.json`.
///
/// All fields except `name` are optional — a plugin can provide any
/// combination of agents, skills, commands, hooks, MCP servers, and tools.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    /// Plugin name (required). Should be kebab-case.
    pub name: String,
    /// Semantic version string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Author information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<PluginAuthor>,

    // --- Component paths (relative to plugin root, must start with "./") ---
    /// Path to agent definition markdown files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents: Option<String>,
    /// Path to skill directories/files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills: Option<String>,
    /// Path to command markdown files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commands: Option<String>,
    /// Path to compiled tool dylibs (.arawn_tool files).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<String>,

    // --- Inline or path-based config ---
    /// Hook configuration — either inline HookConfig or a string path to hooks.json.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_hooks_field")]
    pub hooks: Option<HooksField>,

    /// MCP server configurations keyed by server name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<HashMap<String, McpServerDef>>,

    // --- Settings ---
    /// Settings to merge into the engine's settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<serde_json::Value>,

    /// User-configurable fields declared by this plugin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_config: Option<HashMap<String, UserConfigField>>,
}

/// Author information for a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAuthor {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// MCP server definition within a plugin manifest.
///
/// Unlike `McpServerConfig` from arawn-mcp (which has `name` baked in),
/// the name comes from the HashMap key in `mcpServers`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerDef {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// A user-configurable field declared in the plugin manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfigField {
    /// Field type: "string", "number", "boolean", "directory", "file".
    #[serde(rename = "type")]
    pub field_type: String,
    /// Display label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Help text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether this field is required.
    #[serde(default)]
    pub required: bool,
    /// Default value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

/// The `hooks` field can be either an inline HookConfig or a path string.
#[derive(Debug, Clone, Serialize)]
pub enum HooksField {
    /// Inline hook configuration.
    Inline(HookConfig),
    /// Path to a hooks.json file (relative to plugin root).
    Path(String),
}

fn deserialize_hooks_field<'de, D>(deserializer: D) -> Result<Option<HooksField>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(serde_json::Value::String(s)) => Ok(Some(HooksField::Path(s))),
        Some(obj @ serde_json::Value::Object(_)) => {
            let config: HookConfig =
                serde_json::from_value(obj).map_err(serde::de::Error::custom)?;
            Ok(Some(HooksField::Inline(config)))
        }
        Some(other) => Err(serde::de::Error::custom(format!(
            "hooks field must be a string path or object, got {}",
            other
        ))),
    }
}

/// Structured error from manifest validation.
#[derive(Debug, Clone)]
pub enum PluginError {
    /// A required field is missing.
    MissingField(String),
    /// A path field has an invalid value (doesn't start with "./").
    InvalidPath { field: String, path: String },
    /// JSON parse error.
    ParseError(String),
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginError::MissingField(field) => write!(f, "missing required field: {field}"),
            PluginError::InvalidPath { field, path } => {
                write!(f, "invalid path for '{field}': '{path}' (must start with \"./\")")
            }
            PluginError::ParseError(msg) => write!(f, "parse error: {msg}"),
        }
    }
}

impl PluginManifest {
    /// Load a manifest from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, PluginError> {
        serde_json::from_str(json).map_err(|e| PluginError::ParseError(e.to_string()))
    }

    /// Load a manifest from a file path.
    pub fn from_file(path: &std::path::Path) -> Result<Self, PluginError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| PluginError::ParseError(e.to_string()))?;
        Self::from_json(&content)
    }

    /// Load a manifest from a plugin directory.
    ///
    /// Checks `.claude-plugin/plugin.json` first (Claude Code format),
    /// then falls back to `plugin.json` at the root.
    pub fn from_dir(dir: &std::path::Path) -> Result<Self, PluginError> {
        let claude_path = dir.join(".claude-plugin").join("plugin.json");
        if claude_path.exists() {
            return Self::from_file(&claude_path);
        }
        let root_path = dir.join("plugin.json");
        if root_path.exists() {
            return Self::from_file(&root_path);
        }
        Err(PluginError::ParseError(format!(
            "no plugin.json found in {} (checked .claude-plugin/plugin.json and plugin.json)",
            dir.display()
        )))
    }

    /// Validate the manifest and return any errors found.
    pub fn validate(&self) -> Vec<PluginError> {
        let mut errors = Vec::new();

        if self.name.is_empty() {
            errors.push(PluginError::MissingField("name".into()));
        }

        // Validate component paths start with "./"
        for (field, path) in self.component_paths() {
            if !path.starts_with("./") {
                errors.push(PluginError::InvalidPath {
                    field: field.into(),
                    path: path.into(),
                });
            }
        }

        // Validate hooks path if it's a string
        if let Some(HooksField::Path(ref p)) = self.hooks {
            if !p.starts_with("./") {
                errors.push(PluginError::InvalidPath {
                    field: "hooks".into(),
                    path: p.clone(),
                });
            }
        }

        errors
    }

    /// Get all component path fields that are set.
    fn component_paths(&self) -> Vec<(&str, &str)> {
        let mut paths = Vec::new();
        if let Some(ref p) = self.agents {
            paths.push(("agents", p.as_str()));
        }
        if let Some(ref p) = self.skills {
            paths.push(("skills", p.as_str()));
        }
        if let Some(ref p) = self.commands {
            paths.push(("commands", p.as_str()));
        }
        if let Some(ref p) = self.tools {
            paths.push(("tools", p.as_str()));
        }
        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_manifest() {
        let json = r#"{
            "name": "my-plugin",
            "version": "1.0.0",
            "description": "A test plugin",
            "author": { "name": "Test Author", "email": "test@example.com" },
            "agents": "./agents",
            "skills": "./skills",
            "commands": "./commands",
            "tools": "./tools",
            "hooks": "./hooks.json",
            "mcpServers": {
                "sqlite": { "command": "uvx", "args": ["mcp-server-sqlite"] }
            },
            "userConfig": {
                "API_KEY": {
                    "type": "string",
                    "title": "API Key",
                    "description": "Your API key",
                    "required": true
                }
            }
        }"#;

        let manifest = PluginManifest::from_json(json).unwrap();
        assert_eq!(manifest.name, "my-plugin");
        assert_eq!(manifest.version.as_deref(), Some("1.0.0"));
        assert_eq!(manifest.description.as_deref(), Some("A test plugin"));
        assert_eq!(manifest.author.as_ref().unwrap().name, "Test Author");
        assert_eq!(manifest.agents.as_deref(), Some("./agents"));
        assert_eq!(manifest.skills.as_deref(), Some("./skills"));
        assert_eq!(manifest.commands.as_deref(), Some("./commands"));
        assert_eq!(manifest.tools.as_deref(), Some("./tools"));
        assert!(matches!(manifest.hooks, Some(HooksField::Path(ref p)) if p == "./hooks.json"));

        let mcp = manifest.mcp_servers.as_ref().unwrap();
        assert_eq!(mcp.get("sqlite").unwrap().command, "uvx");

        let uc = manifest.user_config.as_ref().unwrap();
        let key = uc.get("API_KEY").unwrap();
        assert_eq!(key.field_type, "string");
        assert!(key.required);
    }

    #[test]
    fn parse_minimal_manifest() {
        let json = r#"{ "name": "minimal" }"#;
        let manifest = PluginManifest::from_json(json).unwrap();
        assert_eq!(manifest.name, "minimal");
        assert!(manifest.version.is_none());
        assert!(manifest.agents.is_none());
        assert!(manifest.hooks.is_none());
        assert!(manifest.mcp_servers.is_none());
    }

    #[test]
    fn parse_hooks_inline() {
        let json = r#"{
            "name": "test",
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [{ "type": "command", "command": "exit 0" }]
                    }
                ]
            }
        }"#;
        let manifest = PluginManifest::from_json(json).unwrap();
        match manifest.hooks {
            Some(HooksField::Inline(config)) => {
                assert!(!config.is_empty());
            }
            other => panic!("expected Inline hooks, got {other:?}"),
        }
    }

    #[test]
    fn parse_hooks_path() {
        let json = r#"{ "name": "test", "hooks": "./hooks.json" }"#;
        let manifest = PluginManifest::from_json(json).unwrap();
        assert!(matches!(manifest.hooks, Some(HooksField::Path(ref p)) if p == "./hooks.json"));
    }

    #[test]
    fn validate_missing_name() {
        let manifest = PluginManifest {
            name: String::new(),
            ..Default::default()
        };
        let errors = manifest.validate();
        assert!(errors.iter().any(|e| matches!(e, PluginError::MissingField(f) if f == "name")));
    }

    #[test]
    fn validate_invalid_paths() {
        let manifest = PluginManifest {
            name: "test".into(),
            agents: Some("agents".into()), // missing "./"
            skills: Some("./skills".into()), // ok
            ..Default::default()
        };
        let errors = manifest.validate();
        assert_eq!(errors.len(), 1);
        assert!(matches!(&errors[0], PluginError::InvalidPath { field, .. } if field == "agents"));
    }

    #[test]
    fn validate_invalid_hooks_path() {
        let manifest = PluginManifest {
            name: "test".into(),
            hooks: Some(HooksField::Path("hooks.json".into())), // missing "./"
            ..Default::default()
        };
        let errors = manifest.validate();
        assert_eq!(errors.len(), 1);
        assert!(matches!(&errors[0], PluginError::InvalidPath { field, .. } if field == "hooks"));
    }

    #[test]
    fn validate_valid_manifest() {
        let manifest = PluginManifest {
            name: "my-plugin".into(),
            agents: Some("./agents".into()),
            skills: Some("./skills".into()),
            hooks: Some(HooksField::Path("./hooks.json".into())),
            ..Default::default()
        };
        let errors = manifest.validate();
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_error_on_invalid_json() {
        let result = PluginManifest::from_json("not json {{{");
        assert!(matches!(result, Err(PluginError::ParseError(_))));
    }

    #[test]
    fn mcp_server_with_env() {
        let json = r#"{
            "name": "test",
            "mcpServers": {
                "github": {
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-github"],
                    "env": { "GITHUB_TOKEN": "${user_config.GITHUB_TOKEN}" }
                }
            }
        }"#;
        let manifest = PluginManifest::from_json(json).unwrap();
        let mcp = manifest.mcp_servers.as_ref().unwrap();
        let gh = mcp.get("github").unwrap();
        assert_eq!(gh.command, "npx");
        assert_eq!(
            gh.env.get("GITHUB_TOKEN").unwrap(),
            "${user_config.GITHUB_TOKEN}"
        );
    }

    #[test]
    fn user_config_with_default() {
        let json = r#"{
            "name": "test",
            "userConfig": {
                "PORT": {
                    "type": "number",
                    "title": "Port",
                    "required": false,
                    "default": 8080
                }
            }
        }"#;
        let manifest = PluginManifest::from_json(json).unwrap();
        let uc = manifest.user_config.as_ref().unwrap();
        let port = uc.get("PORT").unwrap();
        assert_eq!(port.field_type, "number");
        assert!(!port.required);
        assert_eq!(port.default, Some(serde_json::json!(8080)));
    }
}
