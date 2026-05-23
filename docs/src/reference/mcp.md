# MCP (Model Context Protocol)

*Reference. `[[mcp.servers]]` config + stdio connection model + how MCP tools appear to the agent.*

arawn talks to MCP servers over stdio. Each server you configure shows up as a set of `mcp__<server-name>__<tool>` tools that the agent can call alongside built-ins.

Source: `crates/arawn-mcp/`.

## Configuration

Two ways to declare an MCP server:

1. `[[mcp.servers]]` entries in `arawn.toml` (system-wide).
2. The `mcp_servers` field in a plugin's `plugin.json` (per-plugin; merged at load).

### `arawn.toml` format

```toml
[[mcp.servers]]
name = "sqlite"
command = "uvx"
args = ["mcp-server-sqlite", "--db", "test.db"]

[[mcp.servers]]
name = "github"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
env = { GITHUB_TOKEN = "${GITHUB_TOKEN}" }   # ${VAR} resolves from arawn's parent env at spawn time
enabled = false
```

Fields:

| Field | Type | Default | Description |
|---|---|---|---|
| `name` | string | required | Unique server name. Used in tool naming: `mcp__<name>__<tool>`. |
| `command` | string | required | Command to spawn the server process. |
| `args` | list&lt;string&gt; | `[]` | Arguments for the command. |
| `env` | map | `{}` | Environment variables for the spawned process. `${VAR}` references are resolved from arawn's own environment at spawn time (see [Env-var substitution](#env-var-substitution)). |
| `enabled` | bool | `true` | When `false`, the entry is parsed but the server isn't started. |

## How it works

When arawn starts:

1. `crates/arawn-mcp/src/config.rs::load_mcp_config` reads `arawn.toml`.
2. `crates/arawn-mcp/src/manager.rs` spawns each enabled server as a child process with stdio piped.
3. For each server, `crates/arawn-mcp/src/adapter.rs` performs the MCP handshake and discovers the server's tools.
4. Each discovered tool is registered with the engine's `ToolRegistry` under `mcp__<server-name>__<tool>`.
5. The agent sees the tools alongside built-ins.

## Env-var substitution

Within `[[mcp.servers]].env` values (and the equivalent `env` map on plugin-declared servers), `${VAR}` is replaced with the value of `VAR` from arawn's parent environment at server-spawn time. Same rule applies to both `arawn.toml` and `plugin.json`.

- `GITHUB_TOKEN = "${GITHUB_TOKEN}"` — pulls the value from the shell that started arawn.
- `URL = "${PREFIX}/api"` — substitution happens inside a larger string.
- `LITERAL = "\\${NOT_LOOKED_UP}"` — backslash-escape passes the placeholder through unchanged.

If a referenced variable isn't set, the server fails to start with a clear error (`environment variable \`X\` is not set ...`) logged at server level. Substitution is one-pass — values pulled from env aren't re-scanned for further placeholders.

Substitution is **not** applied to `command` or `args` — only the `env` map. If you need a path resolved from env, set it via env on arawn's side and read it inside the server.

## Tool naming convention

`mcp__<server-name>__<tool>` — double underscore separators. Example: a server named `sqlite` exposing a tool `query` becomes `mcp__sqlite__query`.

The agent calls it like any other tool. The MCP adapter handles JSON-RPC translation.

## Lifecycle and reconnection

If a server process exits, the adapter logs the error. Re-starting it requires either restarting arawn or modifying the configuration to trigger a reload. Auto-reconnection isn't implemented.

## Plugin-declared MCP servers

A plugin's `plugin.json` can include:

```json
{
  "mcp_servers": [
    {
      "name": "my-server",
      "command": "node",
      "args": ["./server.js"],
      "env": {},
      "enabled": true
    }
  ]
}
```

These merge with `arawn.toml` `[[mcp.servers]]` at startup. Same name collision wins by plugin order (last-loaded wins; deterministic but not guaranteed stable across plugin install orderings).

## Inspecting at runtime

```
/mcp
```

Lists connected MCP servers, their connection state, and the tools they expose. Useful for debugging stuck handshakes.

## Permission behavior

MCP tools are classified as `Other` in the permission category system. They follow the active mode's "Other" rule (default = ask). You can override with explicit permission rules:

```toml
[permissions]
allow = ["mcp__sqlite__*"]      # auto-allow all sqlite server tools
deny = ["mcp__github__delete_*"] # never let GitHub server delete
```

See [permissions reference](./permissions.md).

## Caveats

- **stdio only.** TCP/WebSocket MCP transports aren't implemented.
- **No auto-reconnect.** A server crash means the tools disappear until the server is restarted.
- **No sandbox.** MCP server processes are spawned directly. Treat them like trusted dependencies.

## Related

- [Plugins reference](./plugins.md) — plugins can ship MCP server declarations.
- [Config schema reference](./config-schema.md) — `[[mcp.servers]]`.
- [Slash commands reference](./slash-commands.md) — `/mcp`.
- [Agent tools reference](./agent-tools.md) — MCP tools appear here at runtime.
