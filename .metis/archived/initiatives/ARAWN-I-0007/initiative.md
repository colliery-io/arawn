---
id: mcp-model-context-protocol-client
level: initiative
title: "MCP (Model Context Protocol) client integration"
short_code: "ARAWN-I-0007"
created_at: 2026-04-03T01:56:00.442444+00:00
updated_at: 2026-04-04T01:52:29.185830+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: XL
initiative_id: mcp-model-context-protocol-client
---

# MCP (Model Context Protocol) client integration Initiative

## Context

Arawn currently has no MCP support. MCP is the standard protocol for connecting AI assistants to external tool servers — databases, APIs, cloud services, dev tools. Claude Code's MCP client is its biggest integration surface (~3000+ lines), enabling an entire ecosystem of third-party capabilities without building each one natively. Without MCP, arawn is limited to built-in tools and the custom .arawn_tool plugin system.

## Goals & Non-Goals

**Goals:**
- Connect to MCP servers via stdio transport (local processes)
- Connect to MCP servers via SSE/HTTP transport (remote servers)
- Discover and register MCP tools dynamically into the ToolRegistry
- Browse and read MCP resources (ListMcpResources, ReadMcpResource)
- Configure servers via .mcp.json (project-level) and arawn.toml (global)
- Handle server lifecycle (connect, disconnect, reconnect with backoff)

**Non-Goals:**
- OAuth/XAA authentication flows (can be added later)
- MCP prompts-as-commands (nice to have, not MVP)
- Marketplace or remote plugin installation
- WebSocket transport (stdio + SSE/HTTP covers the vast majority)
- Claude.ai proxy transport

## Detailed Design

### Architecture

New `arawn-mcp` crate with:
- `McpClient` — manages a single server connection (stdio or HTTP transport)
- `McpConfig` — parses .mcp.json and arawn.toml server definitions
- `McpToolAdapter` — wraps an MCP tool as an arawn `Tool` impl (like PluginToolAdapter)
- `McpResourceTools` — ListMcpResources and ReadMcpResource built-in tools
- `McpManager` — connects to all configured servers, manages lifecycle, feeds ToolRegistry

### Tool naming
MCP tools registered as `mcp__<server>__<tool>` to avoid collisions with built-in tools.

### Transport layer
- **stdio**: spawn child process, communicate via stdin/stdout JSON-RPC
- **SSE/HTTP**: HTTP POST for requests, SSE for streaming responses

### Configuration
```json
// .mcp.json (project root)
{
  "mcpServers": {
    "sqlite": {
      "command": "uvx",
      "args": ["mcp-server-sqlite", "--db", "app.db"]
    },
    "remote-api": {
      "type": "sse",
      "url": "https://mcp.example.com/sse"
    }
  }
}
```

### Reconnection
Exponential backoff: 1s, 2s, 4s, 8s, 16s (max 30s), max 5 attempts. SSE/HTTP only — stdio servers don't auto-reconnect.

### Reference
Claude Code's implementation: `src/services/mcp/client.ts`, `src/services/mcp/config.ts`, `src/services/mcp/types.ts`

## Decomposition (approximate tasks)

1. MCP config loading (.mcp.json + arawn.toml)
2. stdio transport (spawn process, JSON-RPC over stdin/stdout)
3. SSE/HTTP transport
4. Tool discovery (tools/list) and McpToolAdapter
5. Resource discovery and read (resources/list, resources/read)
6. McpManager — lifecycle, reconnection, ToolRegistry integration
7. Integration with serve mode and CLI mode
8. Tests (unit + integration with a mock MCP server)