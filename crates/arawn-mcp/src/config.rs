//! MCP server configuration — parsed from arawn.toml [[mcp.servers]] entries.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Top-level MCP configuration section from arawn.toml.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpConfig {
    #[serde(default)]
    pub servers: Vec<McpServerConfig>,
}

/// Configuration for a single MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Unique name for this server (used in tool naming: mcp__name__tool).
    pub name: String,
    /// Command to spawn the server process.
    pub command: String,
    /// Arguments to pass to the command.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables to set for the server process.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Whether this server is enabled (default: true).
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Load MCP config from an arawn.toml file.
pub fn load_mcp_config(path: &std::path::Path) -> McpConfig {
    if !path.exists() {
        return McpConfig::default();
    }

    #[derive(Deserialize)]
    struct TomlWrapper {
        #[serde(default)]
        mcp: McpConfig,
    }

    match std::fs::read_to_string(path) {
        Ok(content) => match toml::from_str::<TomlWrapper>(&content) {
            Ok(wrapper) => wrapper.mcp,
            Err(e) => {
                tracing::warn!(error = %e, "failed to parse MCP config from arawn.toml");
                McpConfig::default()
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, "failed to read arawn.toml for MCP config");
            McpConfig::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mcp_config() {
        let toml = r#"
[[mcp.servers]]
name = "sqlite"
command = "uvx"
args = ["mcp-server-sqlite", "--db", "test.db"]

[[mcp.servers]]
name = "github"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
enabled = false
"#;

        #[derive(Deserialize)]
        struct W {
            #[serde(default)]
            mcp: McpConfig,
        }

        let w: W = toml::from_str(toml).unwrap();
        assert_eq!(w.mcp.servers.len(), 2);
        assert_eq!(w.mcp.servers[0].name, "sqlite");
        assert_eq!(w.mcp.servers[0].command, "uvx");
        assert!(w.mcp.servers[0].enabled);
        assert!(!w.mcp.servers[1].enabled);
    }

    #[test]
    fn empty_config() {
        let toml = r#"
[engine]
max_iterations = 20
"#;
        #[derive(Deserialize)]
        struct W {
            #[serde(default)]
            mcp: McpConfig,
        }
        let w: W = toml::from_str(toml).unwrap();
        assert!(w.mcp.servers.is_empty());
    }

    #[test]
    fn config_with_env() {
        let toml = r#"
[[mcp.servers]]
name = "github"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
env = { GITHUB_TOKEN = "ghp_test123" }
"#;
        #[derive(Deserialize)]
        struct W {
            #[serde(default)]
            mcp: McpConfig,
        }
        let w: W = toml::from_str(toml).unwrap();
        assert_eq!(
            w.mcp.servers[0].env.get("GITHUB_TOKEN").unwrap(),
            "ghp_test123"
        );
    }
}
