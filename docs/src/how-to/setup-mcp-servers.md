# Set Up MCP Servers

This guide covers adding, configuring, testing, and managing Model Context Protocol (MCP) servers in Arawn. MCP bridges external tool servers into the agent's tool registry, letting the agent call tools provided by any MCP-compatible server alongside its built-in tools.

## Prerequisites

- Arawn installed and running
- The MCP server binary or HTTP endpoint you want to connect

## Enable MCP

MCP must be enabled in your config file:

```toml
[mcp]
enabled = true
```

Without this, MCP servers are ignored even if configured.

## Add a stdio server

Stdio servers run as child processes. Arawn spawns the command, communicates over stdin/stdout, and shuts the process down when Arawn exits.

### Via the CLI

```bash
arawn mcp add sqlite mcp-server-sqlite -- --db data.db
```

Breaking this down:

- `sqlite` -- the name you assign to this server (used in all subsequent commands)
- `mcp-server-sqlite` -- the command to spawn
- `--` -- separates Arawn flags from arguments passed to the server
- `--db data.db` -- arguments forwarded to `mcp-server-sqlite`

### With environment variables

Pass environment variables the server process needs:

```bash
arawn mcp add search uvx -- mcp-search -e API_KEY=sk-xxx -e DEBUG=true
```

The `-e` / `--env` flag is repeatable.

### Via the config file

```toml
[mcp]
enabled = true

[[mcp.servers]]
name = "sqlite"
transport = "stdio"
command = "mcp-server-sqlite"
args = ["--db", "data.db"]
enabled = true

[[mcp.servers]]
name = "filesystem"
transport = "stdio"
command = "filesystem-mcp"
args = ["--root", "/home/user/projects"]
env = [["DEBUG", "true"]]
enabled = true
```

## Add an HTTP server

HTTP servers run independently (you start them yourself or they run as a service). Arawn connects to them over HTTP.

### Via the CLI

```bash
arawn mcp add api http://localhost:3001 --http
```

Add authentication headers:

```bash
arawn mcp add api http://localhost:3001 --http \
  -H Authorization="Bearer my-token"
```

The `-H` / `--header` flag is repeatable.

### Via the config file

```toml
[[mcp.servers]]
name = "api"
transport = "http"
url = "http://localhost:3001"
headers = [["Authorization", "Bearer my-token"]]
timeout_secs = 30
retries = 3
enabled = true
```

## Configure timeouts and retries

For HTTP servers, you can control connection behavior:

```bash
arawn mcp add remote-api http://api.example.com --http \
  --timeout 60 --retries 5
```

| Option | Default | Description |
|--------|---------|-------------|
| `--timeout` | 30 | Request timeout in seconds |
| `--retries` | 3 | Number of retry attempts for failed requests |

These options only apply to HTTP transport. Stdio servers communicate synchronously over pipes and do not have configurable timeouts at the transport level.

## Test a server

Verify that Arawn can connect to a server, complete the MCP handshake, and list its tools:

```bash
arawn mcp test sqlite
```

Output:

```
Testing connection to MCP server: sqlite
  Command: mcp-server-sqlite

✓ Connected
✓ Initialized: sqlite-mcp v1.2.0
✓ Listed 3 tools

Available tools:
  • query
      Execute SQL queries against the database
  • schema
      Get table schemas and structure
  • insert
      Insert rows into a table

✓ Connection test successful
```

### View full tool schemas

Add `--full` to include the JSON Schema for each tool's parameters:

```bash
arawn mcp test sqlite --full
```

## List configured servers

```bash
arawn mcp list
```

Output:

```
NAME                 TRANSPORT  STATUS     TARGET
────────────────────────────────────────────────────────────────────────────────
sqlite               stdio      enabled    mcp-server-sqlite --db data.db
api                  http       enabled    http://localhost:3001
filesystem           stdio      disabled   filesystem-mcp --root /home/user
```

### Include tool information

Connect to each server and list their tools:

```bash
arawn mcp list --tools
```

## Remove a server

```bash
arawn mcp remove sqlite
```

This removes the server from your config file. If the server was running as a stdio process, it is terminated.

## Disable a server without removing it

### At creation time

```bash
arawn mcp add expensive-api http://api.example.com --http --disabled
```

### In the config file

Set `enabled = false`:

```toml
[[mcp.servers]]
name = "expensive-api"
transport = "http"
url = "http://api.example.com"
enabled = false
```

Disabled servers remain in your configuration but are not started or connected to on launch.

## Manage servers via the REST API

The server exposes MCP management endpoints for runtime control:

### Add a server

```bash
curl -X POST http://localhost:8080/api/v1/mcp/servers \
  -H "Content-Type: application/json" \
  -d '{
    "name": "sqlite",
    "transport": "stdio",
    "command": "mcp-server-sqlite",
    "args": ["--db", "data.db"]
  }'
```

### List servers and tools

```bash
curl http://localhost:8080/api/v1/mcp/servers
```

List tools for a specific server:

```bash
curl http://localhost:8080/api/v1/mcp/servers/sqlite/tools
```

### Remove a server

```bash
curl -X DELETE http://localhost:8080/api/v1/mcp/servers/sqlite
```

## Understand how MCP tools appear to the agent

When an MCP server connects, its tools are registered in the agent's tool registry with a namespaced name:

```
mcp__{server_name}__{tool_name}
```

For example, the `query` tool from the `sqlite` server becomes `mcp__sqlite__query`. This namespacing prevents collisions between tools from different servers or with Arawn's built-in tools.

The agent sees MCP tools and built-in tools in a single unified list. When the LLM decides to call `mcp__sqlite__query`, Arawn:

1. Extracts the server name (`sqlite`) and tool name (`query`)
2. Routes the call to the correct MCP client
3. Returns the result to the LLM

From the agent's perspective, there is no difference between calling a built-in tool and an MCP tool.

## Debug MCP connections

Enable debug logging for the MCP subsystem:

```bash
RUST_LOG=arawn_mcp=debug arawn start
```

This logs every MCP message (initialization, tool discovery, tool calls, and responses), which is useful for diagnosing connection issues or unexpected tool behavior.

## Server lifecycle

MCP servers follow this lifecycle during an Arawn session:

1. **Startup** -- Stdio servers are spawned; HTTP servers are connected to
2. **Initialization** -- The MCP handshake exchanges capabilities
3. **Discovery** -- Arawn queries available tools and registers them
4. **Execution** -- Tools are called as the agent needs them
5. **Shutdown** -- Servers are stopped when Arawn exits (stdio processes are terminated)
