//! MCP (Model Context Protocol) server management endpoints.
//!
//! Provides REST API for runtime MCP server registration and management:
//! - `POST /api/v1/mcp/servers` - Add a new MCP server
//! - `DELETE /api/v1/mcp/servers/:name` - Remove an MCP server
//! - `GET /api/v1/mcp/servers` - List all connected servers and their tools
//! - `GET /api/v1/mcp/servers/:name/tools` - List tools for a specific server

mod types;
pub use types::*;

#[cfg(test)]
mod tests;

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};

use arawn_domain::McpServerConfig;

use crate::auth::Identity;
use crate::error::ServerError;
use crate::state::AppState;

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// POST /api/v1/mcp/servers - Add a new MCP server.
#[utoipa::path(
    post,
    path = "/api/v1/mcp/servers",
    request_body = AddServerRequest,
    responses(
        (status = 201, description = "Server added successfully. If `connect` was true, includes tool count.", body = AddServerResponse),
        (status = 400, description = "Invalid request: missing server name, missing command (stdio), missing URL (http), or server name already exists"),
        (status = 401, description = "Unauthorized — missing or invalid bearer token"),
        (status = 500, description = "MCP feature not enabled in server configuration"),
    ),
    security(("bearer_auth" = [])),
    tag = "mcp"
)]
pub async fn add_server_handler(
    State(state): State<AppState>,
    Extension(_identity): Extension<Identity>,
    Json(request): Json<AddServerRequest>,
) -> Result<(StatusCode, Json<AddServerResponse>), ServerError> {
    let mcp_manager = state
        .mcp_manager()
        .ok_or_else(|| ServerError::Internal("MCP not enabled on this server".to_string()))?;

    // Validate request
    if request.name.is_empty() {
        return Err(ServerError::BadRequest(
            "Server name is required".to_string(),
        ));
    }

    // Check if server already exists
    {
        let manager = mcp_manager.read().await;
        if manager.has_server(&request.name) {
            return Err(ServerError::BadRequest(format!(
                "Server '{}' already exists",
                request.name
            )));
        }
    }

    // Build server config based on transport type
    let transport_type = request.transport.to_lowercase();
    let config = if transport_type == "http" {
        let url = request.url.as_ref().ok_or_else(|| {
            ServerError::BadRequest("URL is required for HTTP transport".to_string())
        })?;

        let mut config = McpServerConfig::http(&request.name, url);

        for (key, value) in &request.headers {
            config = config.with_header(key.clone(), value.clone());
        }

        if let Some(timeout) = request.timeout_secs {
            config = config.with_timeout(std::time::Duration::from_secs(timeout));
        }

        if let Some(retries) = request.retries {
            config = config.with_retries(retries);
        }

        config
    } else {
        // Default to stdio
        if request.command.is_empty() {
            return Err(ServerError::BadRequest(
                "Command is required for stdio transport".to_string(),
            ));
        }

        McpServerConfig::new(&request.name, &request.command)
            .with_args(request.args.clone())
            .with_env(request.env.clone())
    };

    // Add server to manager
    {
        let mut manager = mcp_manager.write().await;
        manager.add_server(config);
    }

    // Optionally connect
    let (connected, tool_count, error) = if request.connect {
        let mut manager = mcp_manager.write().await;
        match manager.connect_server_by_name(&request.name) {
            Ok(()) => {
                // Count tools
                let tools = manager
                    .list_all_tools()
                    .unwrap_or_default()
                    .get(&request.name)
                    .map(|t| t.len())
                    .unwrap_or(0);
                (true, Some(tools), None)
            }
            Err(e) => (false, None, Some(e.to_string())),
        }
    } else {
        (false, None, None)
    };

    Ok((
        StatusCode::CREATED,
        Json(AddServerResponse {
            name: request.name,
            connected,
            tool_count,
            error,
        }),
    ))
}

/// DELETE /api/v1/mcp/servers/:name - Remove an MCP server.
///
/// Disconnects the server (if connected) and removes it from the registry.
#[utoipa::path(
    delete,
    path = "/api/v1/mcp/servers/{name}",
    params(
        ("name" = String, Path, description = "Server name (as provided during registration)"),
    ),
    responses(
        (status = 200, description = "Server disconnected and removed", body = RemoveServerResponse),
        (status = 401, description = "Unauthorized — missing or invalid bearer token"),
        (status = 404, description = "No server registered with this name"),
        (status = 500, description = "MCP feature not enabled in server configuration"),
    ),
    security(("bearer_auth" = [])),
    tag = "mcp"
)]
pub async fn remove_server_handler(
    State(state): State<AppState>,
    Extension(_identity): Extension<Identity>,
    Path(server_name): Path<String>,
) -> Result<Json<RemoveServerResponse>, ServerError> {
    let mcp_manager = state
        .mcp_manager()
        .ok_or_else(|| ServerError::Internal("MCP not enabled on this server".to_string()))?;

    let removed = {
        let mut manager = mcp_manager.write().await;
        manager.remove_server(&server_name)
    };

    if removed {
        Ok(Json(RemoveServerResponse {
            name: server_name,
            removed: true,
        }))
    } else {
        Err(ServerError::NotFound(format!(
            "Server '{}' not found",
            server_name
        )))
    }
}

/// GET /api/v1/mcp/servers - List all MCP servers.
///
/// Returns all registered servers with their connection status and tool names.
#[utoipa::path(
    get,
    path = "/api/v1/mcp/servers",
    responses(
        (status = 200, description = "List of registered servers with connection status and tool counts", body = ListServersResponse),
        (status = 401, description = "Unauthorized — missing or invalid bearer token"),
        (status = 500, description = "MCP feature not enabled in server configuration"),
    ),
    security(("bearer_auth" = [])),
    tag = "mcp"
)]
pub async fn list_servers_handler(
    State(state): State<AppState>,
    Extension(_identity): Extension<Identity>,
) -> Result<Json<ListServersResponse>, ServerError> {
    let mcp_manager = state
        .mcp_manager()
        .ok_or_else(|| ServerError::Internal("MCP not enabled on this server".to_string()))?;

    let manager = mcp_manager.read().await;

    let all_tools = manager.list_all_tools().unwrap_or_default();
    let server_names = manager.server_names();
    let total = server_names.len();
    let connected = manager.connected_count();

    let servers: Vec<ServerInfo> = server_names
        .into_iter()
        .map(|name| {
            let is_connected = manager.is_connected(name);
            let tools = all_tools
                .get(name)
                .map(|t| t.iter().map(|ti| ti.name.clone()).collect())
                .unwrap_or_default();
            let tool_count = if is_connected {
                all_tools.get(name).map(|t| t.len()).unwrap_or(0)
            } else {
                0
            };

            ServerInfo {
                name: name.to_string(),
                connected: is_connected,
                tool_count,
                tools,
            }
        })
        .collect();

    Ok(Json(ListServersResponse {
        servers,
        total,
        connected,
    }))
}

/// GET /api/v1/mcp/servers/:name/tools - List tools for a specific server.
///
/// Returns the tools available on a connected server, including their
/// names, descriptions, and JSON Schema input definitions.
#[utoipa::path(
    get,
    path = "/api/v1/mcp/servers/{name}/tools",
    params(
        ("name" = String, Path, description = "Server name (as provided during registration)"),
    ),
    responses(
        (status = 200, description = "List of tools with names, descriptions, and input schemas", body = ListToolsResponse),
        (status = 400, description = "Server exists but is not connected — call `/connect` first"),
        (status = 401, description = "Unauthorized — missing or invalid bearer token"),
        (status = 404, description = "No server registered with this name"),
        (status = 500, description = "MCP feature not enabled in server configuration"),
    ),
    security(("bearer_auth" = [])),
    tag = "mcp"
)]
pub async fn list_server_tools_handler(
    State(state): State<AppState>,
    Extension(_identity): Extension<Identity>,
    Path(server_name): Path<String>,
) -> Result<Json<ListToolsResponse>, ServerError> {
    let mcp_manager = state
        .mcp_manager()
        .ok_or_else(|| ServerError::Internal("MCP not enabled on this server".to_string()))?;

    let manager = mcp_manager.read().await;

    // Check if server exists
    if !manager.has_server(&server_name) {
        return Err(ServerError::NotFound(format!(
            "Server '{}' not found",
            server_name
        )));
    }

    // Check if server is connected
    if !manager.is_connected(&server_name) {
        return Err(ServerError::BadRequest(format!(
            "Server '{}' is not connected",
            server_name
        )));
    }

    // Get client and list tools
    let client = manager.get_client(&server_name).ok_or_else(|| {
        ServerError::Internal(format!("Failed to get client for '{}'", server_name))
    })?;

    let tools = client
        .list_tools()
        .map_err(|e| ServerError::Internal(format!("Failed to list tools: {}", e)))?;

    let tool_infos: Vec<ToolInfo> = tools
        .into_iter()
        .map(|t| ToolInfo {
            name: t.name,
            description: t.description,
            input_schema: t.input_schema,
        })
        .collect();

    Ok(Json(ListToolsResponse {
        server: server_name,
        tools: tool_infos,
    }))
}

/// POST /api/v1/mcp/servers/:name/connect - Connect to a specific server.
///
/// Establishes the transport connection and discovers available tools.
/// Returns 200 immediately if the server is already connected.
#[utoipa::path(
    post,
    path = "/api/v1/mcp/servers/{name}/connect",
    params(
        ("name" = String, Path, description = "Server name (as provided during registration)"),
    ),
    responses(
        (status = 200, description = "Server connected (or was already connected)"),
        (status = 401, description = "Unauthorized — missing or invalid bearer token"),
        (status = 404, description = "No server registered with this name"),
        (status = 500, description = "MCP feature not enabled, or transport connection failed"),
    ),
    security(("bearer_auth" = [])),
    tag = "mcp"
)]
pub async fn connect_server_handler(
    State(state): State<AppState>,
    Extension(_identity): Extension<Identity>,
    Path(server_name): Path<String>,
) -> Result<StatusCode, ServerError> {
    let mcp_manager = state
        .mcp_manager()
        .ok_or_else(|| ServerError::Internal("MCP not enabled on this server".to_string()))?;

    let mut manager = mcp_manager.write().await;

    if !manager.has_server(&server_name) {
        return Err(ServerError::NotFound(format!(
            "Server '{}' not found",
            server_name
        )));
    }

    if manager.is_connected(&server_name) {
        return Ok(StatusCode::OK); // Already connected
    }

    manager
        .connect_server_by_name(&server_name)
        .map_err(|e| ServerError::Internal(format!("Failed to connect: {}", e)))?;

    Ok(StatusCode::OK)
}

/// POST /api/v1/mcp/servers/:name/disconnect - Disconnect from a specific server.
///
/// Shuts down the transport connection. The server remains registered and
/// can be reconnected later via `/connect`.
#[utoipa::path(
    post,
    path = "/api/v1/mcp/servers/{name}/disconnect",
    params(
        ("name" = String, Path, description = "Server name (as provided during registration)"),
    ),
    responses(
        (status = 200, description = "Server disconnected (or was already disconnected)"),
        (status = 401, description = "Unauthorized — missing or invalid bearer token"),
        (status = 404, description = "No server registered with this name"),
        (status = 500, description = "MCP feature not enabled in server configuration"),
    ),
    security(("bearer_auth" = [])),
    tag = "mcp"
)]
pub async fn disconnect_server_handler(
    State(state): State<AppState>,
    Extension(_identity): Extension<Identity>,
    Path(server_name): Path<String>,
) -> Result<StatusCode, ServerError> {
    let mcp_manager = state
        .mcp_manager()
        .ok_or_else(|| ServerError::Internal("MCP not enabled on this server".to_string()))?;

    let mut manager = mcp_manager.write().await;

    if !manager.has_server(&server_name) {
        return Err(ServerError::NotFound(format!(
            "Server '{}' not found",
            server_name
        )));
    }

    manager.shutdown_server(&server_name);
    Ok(StatusCode::OK)
}
