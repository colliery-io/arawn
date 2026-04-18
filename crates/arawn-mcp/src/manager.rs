//! McpManager — connects to configured MCP servers, discovers tools,
//! registers them in the ToolRegistry, and handles reconnection.

use std::collections::HashMap;
use std::sync::Arc;

use rmcp::ClientHandler;
use rmcp::model::{ClientInfo, Implementation, Tool as McpTool};
use rmcp::service::{RoleClient, RunningService};
use tokio::process::Command;
use tracing::{error, info, warn};

use arawn_tool::ToolRegistry;

use crate::adapter::McpToolAdapter;
use crate::config::McpServerConfig;

/// Handler for MCP client notifications. Default impls are fine for now.
struct ArawnClientHandler;

impl ClientHandler for ArawnClientHandler {
    fn get_info(&self) -> ClientInfo {
        ClientInfo::new(
            Default::default(),
            Implementation::new("arawn", env!("CARGO_PKG_VERSION")),
        )
    }
}

/// State of a connected MCP server.
struct ConnectedServer {
    config: McpServerConfig,
    _service: RunningService<RoleClient, ArawnClientHandler>,
    tools: Vec<McpTool>,
    /// Server-provided instructions (from InitializeResult), if any.
    instructions: Option<String>,
}

/// Manages all MCP server connections.
pub struct McpManager {
    servers: HashMap<String, ConnectedServer>,
}

impl Default for McpManager {
    fn default() -> Self {
        Self::new()
    }
}

impl McpManager {
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
        }
    }

    /// Connect to all enabled servers and discover their tools.
    pub async fn connect_all(
        &mut self,
        configs: &[McpServerConfig],
        registry: &Arc<ToolRegistry>,
    ) {
        for config in configs {
            if !config.enabled {
                info!(name = %config.name, "MCP server disabled, skipping");
                continue;
            }
            self.connect_server(config, registry).await;
        }
    }

    /// Connect to a single MCP server. Public for hot-reload.
    pub async fn connect_server(
        &mut self,
        config: &McpServerConfig,
        registry: &Arc<ToolRegistry>,
    ) {
        info!(name = %config.name, command = %config.command, "connecting to MCP server");

        match spawn_and_connect(config).await {
            Ok((service, tools, instructions)) => {
                let peer = service.peer().clone();
                let peer = Arc::new(peer);
                let tool_count = tools.len();

                for mcp_tool in &tools {
                    let adapter = McpToolAdapter::new(&config.name, mcp_tool.clone(), peer.clone());
                    info!(name = %adapter.tool_name(), "registered MCP tool");
                    registry.register(Box::new(adapter));
                }

                if let Some(ref instr) = instructions {
                    info!(name = %config.name, "server provided instructions ({} chars)", instr.len());
                }
                info!(name = %config.name, tools = tool_count, "MCP server connected");

                self.servers.insert(
                    config.name.clone(),
                    ConnectedServer {
                        config: config.clone(),
                        _service: service,
                        tools,
                        instructions,
                    },
                );
            }
            Err(e) => {
                error!(name = %config.name, error = %e, "failed to connect to MCP server");
            }
        }
    }

    /// Disconnect a server and unregister its tools.
    pub fn disconnect_server(&mut self, name: &str, registry: &Arc<ToolRegistry>) {
        if let Some(server) = self.servers.remove(name) {
            let prefix = format!("mcp__{}__", normalize_name(name));
            let removed = registry.unregister_by_prefix(&prefix);
            info!(
                name,
                tools_removed = removed.len(),
                "MCP server disconnected"
            );
            drop(server); // RunningService dropped, process cleaned up
        }
    }

    /// Diff current servers against a new config and connect/disconnect as needed.
    pub async fn sync_servers(
        &mut self,
        configs: &[McpServerConfig],
        registry: &Arc<ToolRegistry>,
    ) {
        let new_names: std::collections::HashSet<String> =
            configs.iter().filter(|c| c.enabled).map(|c| c.name.clone()).collect();
        let current_names: std::collections::HashSet<String> =
            self.servers.keys().cloned().collect();

        // Disconnect removed servers
        for name in current_names.difference(&new_names) {
            self.disconnect_server(name, registry);
        }

        // Connect new servers
        for config in configs {
            if !config.enabled {
                continue;
            }
            if !self.servers.contains_key(&config.name) {
                self.connect_server(config, registry).await;
            }
        }
    }

    /// Attempt to reconnect a failed server with exponential backoff.
    pub async fn reconnect(
        &mut self,
        server_name: &str,
        registry: &Arc<ToolRegistry>,
    ) -> bool {
        let config = match self.servers.get(server_name) {
            Some(s) => s.config.clone(),
            None => return false,
        };

        self.servers.remove(server_name);

        const MAX_ATTEMPTS: u32 = 5;
        let mut delay_ms: u64 = 1000;

        for attempt in 1..=MAX_ATTEMPTS {
            warn!(name = %config.name, attempt, delay_ms, "reconnecting MCP server");
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;

            match spawn_and_connect(&config).await {
                Ok((service, tools, instructions)) => {
                    let peer = Arc::new(service.peer().clone());

                    for mcp_tool in &tools {
                        let adapter = McpToolAdapter::new(&config.name, mcp_tool.clone(), peer.clone());
                        registry.register(Box::new(adapter));
                    }

                    info!(name = %config.name, attempt, tools = tools.len(), "MCP server reconnected");

                    self.servers.insert(config.name.clone(), ConnectedServer {
                        config: config.clone(),
                        _service: service,
                        tools,
                        instructions,
                    });
                    return true;
                }
                Err(e) => {
                    warn!(name = %config.name, attempt, error = %e, "reconnection failed");
                    delay_ms = (delay_ms * 2).min(30_000);
                }
            }
        }

        error!(name = %config.name, "reconnection failed after {MAX_ATTEMPTS} attempts");
        false
    }

    /// Get the names of all connected servers.
    pub fn connected_servers(&self) -> Vec<&str> {
        self.servers.keys().map(|s| s.as_str()).collect()
    }

    /// Get tool count across all servers.
    pub fn tool_count(&self) -> usize {
        self.servers.values().map(|s| s.tools.len()).sum()
    }

    /// Generate a system prompt section describing connected MCP servers and their tools.
    pub fn system_prompt(&self) -> String {
        if self.servers.is_empty() {
            return String::new();
        }

        let mut prompt = String::from("# MCP Server Instructions\n\nThe following MCP servers have provided instructions for how to use their tools and resources:\n\n");

        for (name, server) in &self.servers {
            prompt.push_str(&format!("## {name}\n\n"));

            // Server-provided instructions (most valuable context)
            if let Some(ref instructions) = server.instructions {
                let capped = if instructions.len() > 2048 {
                    format!("{}… [truncated]", &instructions[..2048])
                } else {
                    instructions.clone()
                };
                prompt.push_str(&capped);
                prompt.push_str("\n\n");
            }

            // List available tools
            prompt.push_str("Available tools:\n");
            for tool in &server.tools {
                let normalized_server = normalize_name(name);
                let normalized_tool = normalize_name(&tool.name);
                let full_name = format!("mcp__{normalized_server}__{normalized_tool}");
                let desc = tool.description.as_deref().unwrap_or("");
                let capped_desc = if desc.len() > 200 {
                    format!("{}…", &desc[..200])
                } else {
                    desc.to_string()
                };
                prompt.push_str(&format!("- `{full_name}`: {capped_desc}\n"));
            }
            prompt.push('\n');
        }

        prompt
    }
}

fn normalize_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect()
}

/// Spawn an MCP server process, connect via stdio, initialize, and discover tools.
async fn spawn_and_connect(
    config: &McpServerConfig,
) -> Result<
    (
        RunningService<RoleClient, ArawnClientHandler>,
        Vec<McpTool>,
        Option<String>,
    ),
    Box<dyn std::error::Error + Send + Sync>,
> {
    let mut cmd = Command::new(&config.command);
    cmd.args(&config.args);
    for (key, val) in &config.env {
        cmd.env(key, val);
    }

    let transport = rmcp::transport::child_process::TokioChildProcess::new(cmd)?;
    let service = rmcp::service::serve_client(ArawnClientHandler, transport).await?;

    // Get server instructions from the initialize result
    let instructions = service
        .peer()
        .peer_info()
        .and_then(|info| info.instructions.clone());

    let tools = service.peer().list_all_tools().await?;

    Ok((service, tools, instructions))
}
