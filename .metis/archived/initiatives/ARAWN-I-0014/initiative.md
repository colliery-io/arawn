---
id: slash-commands-tui-command-system
level: initiative
title: "Slash commands — TUI command system with inventory queries, skill invocation, and autocomplete"
short_code: "ARAWN-I-0014"
created_at: 2026-04-04T17:45:56.775292+00:00
updated_at: 2026-04-05T12:51:07.832098+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: slash-commands-tui-command-system
---

# Slash commands — TUI command system with inventory queries, skill invocation, and autocomplete Initiative

## Context

Arawn's TUI sends all input directly to the server as chat messages. There's no way for users to interact with the system itself — query what plugins/skills/agents/MCP servers are available, invoke skills directly, or get help. The server already has rich registries (`ToolRegistry`, `SkillRegistry`, `PluginRegistry`, `McpManager`) but none of these are exposed to the user.

Claude Code has slash commands (`/help`, `/compact`, `/permissions`, etc.) and skill invocation (`/skill-name`). We need parity.

### Current Architecture
- **TUI input**: `input_buffer` → `Submit` action → `send_message` WS method → server processes as user message
- **WS protocol**: JSON-RPC over WebSocket, methods: `send_message`, `list_sessions`, `create_session`, `load_session`, etc.
- **Server registries**: `ToolRegistry`, `SkillRegistry`, `PluginRegistry` on `LocalService`, `McpManager` for MCP servers
- **Skills**: Already have `user_invocable` flag, name, description — natural slash command candidates
- **No command parsing exists** — all input is treated as chat content

## Goals & Non-Goals

**Goals:**
- Parse `/command` prefix in TUI input, intercept before sending as chat
- Inventory queries (`/plugins`, `/skills`, `/agents`, `/mcp`, `/tools`) that show available items in a navigable modal
- Skill invocation (`/skill-name args`) — inject skill prompt into chat as a user message to the LLM
- Cache available command names (built-in + user-invocable skills) for autocomplete
- Tab-completion on `/` prefix showing available commands
- New WS protocol methods for querying server inventories
- Server-side handlers that query registries and return structured data

**Non-Goals:**
- Interactive plugin management (install/uninstall via slash commands)
- Command history / recall
- Custom user-defined commands (beyond skills)
- Piping command output or command chaining

## Architecture

### Overview

Three layers: **TUI command parser** → **WS protocol methods** → **Server inventory handlers**

```
User types "/skills"
  │
  ├─ TUI detects "/" prefix
  ├─ Parses command name + args
  ├─ Looks up in CommandRegistry (built-in + cached skill names)
  │
  ├─ If inventory command:
  │   ├─ Send WS method (e.g. "list_skills")
  │   ├─ Server queries SkillRegistry
  │   ├─ Returns structured data
  │   └─ TUI renders in navigable modal
  │
  ├─ If skill invocation:
  │   ├─ Send WS method "invoke_skill" with name + args
  │   ├─ Server loads skill, injects as user message context
  │   └─ Streams response back as normal chat
  │
  └─ If built-in (e.g. /help, /clear):
      └─ Handle client-side in TUI
```

### Command Registry

```rust
enum SlashCommand {
    BuiltIn(BuiltInCommand),   // /help, /clear, /compact
    Inventory(InventoryKind),   // /plugins, /skills, /agents, /mcp, /tools
    Skill(String),              // /skill-name — from cached user_invocable skills
}

enum InventoryKind {
    Plugins,
    Skills,
    Agents,
    McpServers,
    Tools,
}
```

### WS Protocol Additions

New methods on the WebSocket protocol:

| Method | Params | Returns |
|--------|--------|---------|
| `list_available_commands` | none | `Vec<CommandInfo>` (name, description, kind) |
| `query_inventory` | `{ kind: "plugins"\|"skills"\|... }` | `InventoryResult` (items with name, description, source, status) |
| `invoke_skill` | `{ name, args, session_id }` | Streams `EngineEvent`s (same as `send_message`) |

### Autocomplete Flow

```
On TUI startup / session load:
  → WS call "list_available_commands"
  → Cache command names + descriptions locally

When user types "/":
  → Show autocomplete dropdown filtered by typed prefix
  → Tab to accept, Escape to dismiss

On plugin/skill hot-reload:
  → Server pushes "commands_changed" event
  → TUI refreshes cache
```

### Inventory Modal

Inventory results render in a modal overlay (reusing existing modal infrastructure):
- Scrollable list with name, description, source columns
- Up/Down navigation, Escape to close
- For skills: Enter to invoke directly

## Detailed Design

### TUI Changes

1. **Command parser** (`command.rs`): Parse `/name args` from input buffer, resolve against registry
2. **Autocomplete widget** (`autocomplete.rs`): Dropdown below input that filters commands on keystroke
3. **Inventory modal** (`inventory_modal.rs`): Scrollable table modal for query results
4. **Event loop**: Intercept `Submit` when input starts with `/`, route to command handler instead of `send_message`
5. **App state**: Add `cached_commands: Vec<CommandInfo>`, `autocomplete_state: Option<AutocompleteState>`

### Server Changes

1. **Inventory handlers**: New functions that query `ToolRegistry`, `SkillRegistry`, `PluginRegistry`, `McpManager` and return structured items
2. **WS method routing**: Add `list_available_commands`, `query_inventory`, `invoke_skill` to WS handler
3. **Skill invocation**: Load skill definition, format with args, create user message, run through engine
4. **Change notification**: Push `commands_changed` event when registries update (hook into plugin hot-reload)

### Data Types

```rust
struct CommandInfo {
    name: String,           // "skills", "help", "metis-ralph"
    description: String,    // One-line description
    kind: CommandKind,      // BuiltIn, Inventory, Skill
}

struct InventoryItem {
    name: String,
    description: String,
    source: String,         // "built-in", "plugin:metis", "mcp:github"
    status: Option<String>, // "enabled", "connected", etc.
}

struct InventoryResult {
    kind: String,
    items: Vec<InventoryItem>,
}
```

## Testing Strategy

- **Unit tests**: Command parser (slash prefix detection, name/arg splitting, resolution)
- **Unit tests**: Autocomplete filtering logic
- **Unit tests**: Server inventory handlers (mock registries → structured output)
- **Integration tests**: WS round-trip for inventory queries
- **Snapshot tests**: Inventory modal rendering, autocomplete dropdown rendering

## Alternatives Considered

- **Handle everything client-side**: TUI could parse and render without server round-trips. Rejected — the server holds the registries; duplicating that state is fragile and breaks with hot-reload.
- **Send `/command` as user message, let LLM handle it**: This is what Claude Code partially does (skills are injected into the conversation). Rejected for inventory queries — querying available tools shouldn't cost API tokens or require LLM processing.
- **Separate command panel instead of modal**: A persistent sidebar or panel for command output. Rejected for now — modal is simpler and reuses existing infrastructure. Can revisit later.

## Implementation Plan

**Phase 1 — Protocol & server inventory handlers**: Add WS methods, query registries, return structured data. This is the foundation everything else depends on.

**Phase 2 — TUI command parser & routing**: Parse `/` prefix, resolve commands, route to handlers. Wire up inventory queries to display results (simple text first, modal later).

**Phase 3 — Skill invocation**: `/skill-name args` sends to server, streams response as normal chat turn.

**Phase 4 — Autocomplete & inventory modal**: Cache commands on startup, show filtered dropdown on `/` prefix, navigable modal for inventory results.

**Phase 5 — Change notifications**: Server pushes `commands_changed` when registries update, TUI refreshes cache.