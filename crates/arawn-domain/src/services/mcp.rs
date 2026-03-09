//! MCP service for tool discovery and management.
//!
//! The MCP service provides access to Model Context Protocol servers
//! and their tools.

use std::sync::Arc;

use arawn_mcp::{McpManager, McpServerConfig};
use tokio::sync::RwLock;
use tracing::debug;

use crate::error::{DomainError, Result};

/// Shared MCP manager type.
pub type SharedMcpManager = Arc<RwLock<McpManager>>;

/// Information about an MCP server.
#[derive(Debug, Clone)]
pub struct McpServerInfo {
    /// Server name.
    pub name: String,
    /// Server command.
    pub command: String,
    /// Whether the server is connected.
    pub connected: bool,
    /// Number of tools provided.
    pub tool_count: usize,
}

/// Information about an MCP tool.
#[derive(Debug, Clone)]
pub struct McpToolInfo {
    /// Tool name.
    pub name: String,
    /// Tool description.
    pub description: Option<String>,
    /// Server that provides this tool.
    pub server: String,
}

/// MCP service for tool discovery and management.
#[derive(Clone)]
pub struct McpService {
    manager: Option<SharedMcpManager>,
}

impl McpService {
    /// Create a new MCP service.
    pub fn new(manager: Option<SharedMcpManager>) -> Self {
        Self { manager }
    }

    /// Check if MCP is enabled.
    pub fn is_enabled(&self) -> bool {
        self.manager.is_some()
    }

    /// Get the MCP manager.
    pub fn manager(&self) -> Option<&SharedMcpManager> {
        self.manager.as_ref()
    }

    /// List all configured MCP server names.
    pub async fn list_server_names(&self) -> Result<Vec<String>> {
        let manager = self
            .manager
            .as_ref()
            .ok_or_else(|| DomainError::Mcp("MCP not enabled".to_string()))?;

        let guard = manager.read().await;
        let names: Vec<String> = guard.server_names().iter().map(|s| s.to_string()).collect();

        debug!(server_count = names.len(), "Listed MCP servers");
        Ok(names)
    }

    /// Check if a server is connected.
    pub async fn is_server_connected(&self, name: &str) -> Result<bool> {
        let manager = self
            .manager
            .as_ref()
            .ok_or_else(|| DomainError::Mcp("MCP not enabled".to_string()))?;

        let guard = manager.read().await;
        Ok(guard.is_connected(name))
    }

    /// Add a new MCP server configuration.
    pub async fn add_server(&self, config: McpServerConfig) -> Result<()> {
        let manager = self
            .manager
            .as_ref()
            .ok_or_else(|| DomainError::Mcp("MCP not enabled".to_string()))?;

        let name = config.name.clone();
        let mut guard = manager.write().await;
        guard.add_server(config);

        debug!(server = %name, "Added MCP server");
        Ok(())
    }

    /// Remove an MCP server.
    pub async fn remove_server(&self, name: &str) -> Result<bool> {
        let manager = self
            .manager
            .as_ref()
            .ok_or_else(|| DomainError::Mcp("MCP not enabled".to_string()))?;

        let mut guard = manager.write().await;
        let removed = guard.remove_server(name);

        if removed {
            debug!(server = name, "Removed MCP server");
        }
        Ok(removed)
    }

    /// Connect to all configured MCP servers.
    pub async fn connect_all(&self) -> Result<()> {
        let manager = self
            .manager
            .as_ref()
            .ok_or_else(|| DomainError::Mcp("MCP not enabled".to_string()))?;

        let mut guard = manager.write().await;
        guard
            .connect_all()
            .map_err(|e| DomainError::Mcp(e.to_string()))?;

        debug!("Connected to all MCP servers");
        Ok(())
    }

    /// Shutdown all MCP server connections.
    pub async fn shutdown_all(&self) -> Result<()> {
        let manager = self
            .manager
            .as_ref()
            .ok_or_else(|| DomainError::Mcp("MCP not enabled".to_string()))?;

        let mut guard = manager.write().await;
        guard
            .shutdown_all()
            .map_err(|e| DomainError::Mcp(e.to_string()))?;

        debug!("Shutdown all MCP servers");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_service_disabled() {
        let service = McpService::new(None);
        assert!(!service.is_enabled());
        assert!(service.manager().is_none());
    }

    #[test]
    fn test_mcp_service_enabled() {
        let manager = Arc::new(RwLock::new(McpManager::new()));
        let service = McpService::new(Some(manager));
        assert!(service.is_enabled());
        assert!(service.manager().is_some());
    }

    #[test]
    fn test_mcp_service_clone() {
        let manager = Arc::new(RwLock::new(McpManager::new()));
        let service = McpService::new(Some(manager));
        let cloned = service.clone();
        assert!(cloned.is_enabled());
    }

    #[tokio::test]
    async fn test_list_server_names_disabled() {
        let service = McpService::new(None);
        let result = service.list_server_names().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("MCP not enabled"));
    }

    #[tokio::test]
    async fn test_list_server_names_empty() {
        let manager = Arc::new(RwLock::new(McpManager::new()));
        let service = McpService::new(Some(manager));
        let names = service.list_server_names().await.unwrap();
        assert!(names.is_empty());
    }

    #[tokio::test]
    async fn test_is_server_connected_disabled() {
        let service = McpService::new(None);
        let result = service.is_server_connected("test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_is_server_connected_unknown() {
        let manager = Arc::new(RwLock::new(McpManager::new()));
        let service = McpService::new(Some(manager));
        let connected = service.is_server_connected("nonexistent").await.unwrap();
        assert!(!connected);
    }

    #[tokio::test]
    async fn test_add_server_disabled() {
        let service = McpService::new(None);
        let config = McpServerConfig::new("test", "echo");
        let result = service.add_server(config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_add_and_list_server() {
        let manager = Arc::new(RwLock::new(McpManager::new()));
        let service = McpService::new(Some(manager));

        let config = McpServerConfig::new("my-server", "echo");
        service.add_server(config).await.unwrap();

        let names = service.list_server_names().await.unwrap();
        assert_eq!(names.len(), 1);
        assert_eq!(names[0], "my-server");
    }

    #[tokio::test]
    async fn test_remove_server_disabled() {
        let service = McpService::new(None);
        let result = service.remove_server("test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_remove_nonexistent_server() {
        let manager = Arc::new(RwLock::new(McpManager::new()));
        let service = McpService::new(Some(manager));
        let removed = service.remove_server("nonexistent").await.unwrap();
        assert!(!removed);
    }

    #[tokio::test]
    async fn test_add_then_remove_server() {
        let manager = Arc::new(RwLock::new(McpManager::new()));
        let service = McpService::new(Some(manager));

        let config = McpServerConfig::new("removable", "echo");
        service.add_server(config).await.unwrap();

        let removed = service.remove_server("removable").await.unwrap();
        assert!(removed);

        let names = service.list_server_names().await.unwrap();
        assert!(names.is_empty());
    }

    #[tokio::test]
    async fn test_connect_all_disabled() {
        let service = McpService::new(None);
        let result = service.connect_all().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_shutdown_all_disabled() {
        let service = McpService::new(None);
        let result = service.shutdown_all().await;
        assert!(result.is_err());
    }

    #[test]
    fn test_mcp_server_info_fields() {
        let info = McpServerInfo {
            name: "test-server".to_string(),
            command: "/usr/bin/test".to_string(),
            connected: true,
            tool_count: 5,
        };
        assert_eq!(info.name, "test-server");
        assert_eq!(info.command, "/usr/bin/test");
        assert!(info.connected);
        assert_eq!(info.tool_count, 5);
    }

    #[test]
    fn test_mcp_server_info_clone() {
        let info = McpServerInfo {
            name: "srv".to_string(),
            command: "cmd".to_string(),
            connected: false,
            tool_count: 0,
        };
        let cloned = info.clone();
        assert_eq!(info.name, cloned.name);
        assert_eq!(info.connected, cloned.connected);
    }

    #[test]
    fn test_mcp_tool_info_fields() {
        let info = McpToolInfo {
            name: "read_file".to_string(),
            description: Some("Read a file".to_string()),
            server: "filesystem".to_string(),
        };
        assert_eq!(info.name, "read_file");
        assert_eq!(info.description.as_deref(), Some("Read a file"));
        assert_eq!(info.server, "filesystem");
    }

    #[test]
    fn test_mcp_tool_info_no_description() {
        let info = McpToolInfo {
            name: "tool".to_string(),
            description: None,
            server: "srv".to_string(),
        };
        assert!(info.description.is_none());
    }

    #[test]
    fn test_mcp_tool_info_clone() {
        let info = McpToolInfo {
            name: "tool".to_string(),
            description: Some("desc".to_string()),
            server: "srv".to_string(),
        };
        let cloned = info.clone();
        assert_eq!(info.name, cloned.name);
        assert_eq!(info.description, cloned.description);
    }

    #[test]
    fn test_mcp_server_info_debug() {
        let info = McpServerInfo {
            name: "srv".to_string(),
            command: "cmd".to_string(),
            connected: true,
            tool_count: 3,
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("srv"));
        assert!(debug.contains("cmd"));
    }

    #[test]
    fn test_mcp_tool_info_debug() {
        let info = McpToolInfo {
            name: "tool".to_string(),
            description: None,
            server: "srv".to_string(),
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("tool"));
    }
}
