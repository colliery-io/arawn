use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request to add a new MCP server.
///
/// ## Transport Types
///
/// ### `stdio` (default)
/// Launches a local process and communicates over stdin/stdout.
/// - **Required:** `command`
/// - **Optional:** `args`, `env`
/// - **Ignored:** `url`, `headers`, `timeout_secs`, `retries`
///
/// ### `http`
/// Connects to a remote MCP server over HTTP.
/// - **Required:** `url`
/// - **Optional:** `headers`, `timeout_secs`, `retries`
/// - **Ignored:** `command`, `args`, `env`
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AddServerRequest {
    /// Unique name for this server. Used as an identifier in all subsequent
    /// operations (list, connect, disconnect, remove).
    pub name: String,

    /// Transport type: `"stdio"` (default) or `"http"`.
    ///
    /// Determines which other fields are required. See the struct-level
    /// documentation for details.
    #[serde(default)]
    #[schema(example = "stdio")]
    pub transport: String,

    /// Command to execute. **Required for `stdio` transport**, ignored for `http`.
    #[serde(default)]
    #[schema(example = "/usr/local/bin/mcp-server")]
    pub command: String,

    /// URL for the server. **Required for `http` transport**, ignored for `stdio`.
    #[serde(default)]
    #[schema(example = "http://localhost:8080/mcp")]
    pub url: Option<String>,

    /// Arguments to pass to the command. Only used with `stdio` transport.
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables as `[key, value]` pairs. Only used with `stdio` transport.
    #[serde(default)]
    #[schema(value_type = Vec<Vec<String>>)]
    pub env: Vec<(String, String)>,

    /// HTTP headers as `[key, value]` pairs. Only used with `http` transport.
    #[serde(default)]
    #[schema(value_type = Vec<Vec<String>>)]
    pub headers: Vec<(String, String)>,

    /// Request timeout in seconds. Only used with `http` transport. Defaults to 30s.
    #[serde(default)]
    pub timeout_secs: Option<u64>,

    /// Number of retries on failure. Only used with `http` transport. Defaults to 3.
    #[serde(default)]
    pub retries: Option<u32>,

    /// Whether to connect immediately after adding. Defaults to `true`.
    ///
    /// When `true`, the server attempts to connect and discover tools before
    /// returning. When `false`, the server is registered but not connected —
    /// use the `/connect` endpoint to connect later.
    #[serde(default = "default_connect")]
    pub connect: bool,
}

fn default_connect() -> bool {
    true
}

/// Response after adding a server.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AddServerResponse {
    /// Server name.
    pub name: String,
    /// Whether the server was connected.
    pub connected: bool,
    /// Number of tools discovered (if connected).
    pub tool_count: Option<usize>,
    /// Error message if connection failed.
    pub error: Option<String>,
}

/// Information about a connected MCP server.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServerInfo {
    /// Server name.
    pub name: String,
    /// Whether the server is connected.
    pub connected: bool,
    /// Number of tools available.
    pub tool_count: usize,
    /// Tool names.
    pub tools: Vec<String>,
}

/// Response for listing servers.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListServersResponse {
    /// List of servers.
    pub servers: Vec<ServerInfo>,
    /// Total number of configured servers.
    pub total: usize,
    /// Total number of connected servers.
    pub connected: usize,
}

/// Information about a tool.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ToolInfo {
    /// Tool name.
    pub name: String,
    /// Tool description.
    pub description: Option<String>,
    /// Input schema (JSON Schema).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Object)]
    pub input_schema: Option<serde_json::Value>,
}

/// Response for listing tools from a server.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListToolsResponse {
    /// Server name.
    pub server: String,
    /// List of tools.
    pub tools: Vec<ToolInfo>,
}

/// Response after removing a server.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RemoveServerResponse {
    /// Server name.
    pub name: String,
    /// Whether the server was removed.
    pub removed: bool,
}
