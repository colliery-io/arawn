# System Overview: Arawn

## Summary

Arawn is a personal agentic assistant built in Rust. It functions as an AI-powered coding assistant similar in concept to Claude Code, providing an agentic loop where user messages are processed by an LLM that can invoke tools (file read/write/edit, shell execution, web search, grep, glob, etc.) in an iterative cycle until a final response is produced. The system supports multiple LLM providers through an OpenAI-compatible abstraction, with Groq as the default provider and Anthropic as a first-class alternative.

The system is organized as a Rust workspace of 13 crates following a layered architecture: core domain types at the bottom, engine and storage in the middle, and UI/binary at the top. It operates in three modes: a WebSocket server (`arawn serve`) that hosts the engine and all tool execution, a terminal UI (`arawn tui` or default) that connects to the server as a client, and a one-shot CLI mode that also connects to the server. This client-server split means the engine always runs inside the server process, and both TUI and CLI are thin WebSocket clients.

The project includes a plugin system (both legacy WASM-based via `fidius` and a newer Claude Code-compatible directory-based format), a permissions system with allow/deny/ask rules, a hooks system for lifecycle event automation, a knowledge-base memory system backed by SQLite with FTS5 and vector search, context compaction via LLM summarization, MCP server integration, a workflow engine (via the `cloacina` crate), and a skill framework for prompt-based reusable workflows.

## Repository Structure

```
arawn/
├── .angreal/                  # Build/test task runner (Python-based, uses Flox for env)
│   ├── task_build.py          # `angreal build workspace [--release]`
│   ├── task_check.py          # Linting/check tasks
│   ├── task_docs.py           # Documentation generation
│   ├── task_test.py           # `angreal test all|unit|integration|coverage`
│   └── workflows/             # CI workflow definitions
├── .claude/                   # Claude Code configuration
├── .flox/                     # Flox environment (Nix-based dev shell)
├── .github/workflows/         # GitHub Actions CI
├── .metis/                    # Project management (Metis plugin)
│   ├── backlog/               # Bug/feature tracking
│   ├── initiatives/           # Project initiatives
│   ├── specifications/        # System specifications
│   ├── vision.md              # Strategic vision document
│   ├── code-index.md          # Auto-generated code symbol index
│   └── config.toml            # Metis configuration
├── backup/                    # Backup/scratch directory
├── crates/                    # Rust workspace members (see below)
├── scripts/                   # Utility scripts
│   └── functional_test.py     # Functional test script
├── vendor/                    # Vendored dependencies
│   └── sandbox-runtime/       # Shell sandboxing runtime (patched crate)
├── Cargo.toml                 # Workspace root
├── Cargo.lock
└── .gitignore
```

### Crate Organization

```
crates/
├── arawn/              # Binary crate: main entrypoint, config, WebSocket server, LocalService
├── arawn-core/         # Domain types: Message, Session, SessionStats, Workstream
├── arawn-embed/        # Embedding providers (local ONNX, API-based)
├── arawn-engine/       # Query engine, tools, permissions, hooks, plugins, skills, compaction
├── arawn-llm/          # LLM client abstraction (Anthropic, Groq, OpenAI-compat, mock, retry)
├── arawn-mcp/          # MCP (Model Context Protocol) server integration
├── arawn-memory/       # Knowledge base: graph-backed entity storage, FTS5, vector search
├── arawn-service/      # Service trait (ArawnService) + transport types (EngineEvent, etc.)
├── arawn-storage/      # Persistence: SQLite metadata + JSONL message files
├── arawn-tests/        # Integration test suite
├── arawn-tool-plugin/  # Plugin interface for WASM tool plugins (via fidius)
├── arawn-tui/          # Terminal UI (ratatui + crossterm)
└── arawn-workflow/     # Workflow engine integration (cloacina DAG runner)
```

## Key Entrypoints

### Binary: `crates/arawn/src/main.rs`

The single binary `arawn` supports multiple modes determined by CLI arguments:

1. **`arawn serve [--port PORT]`** -- Starts the WebSocket server. This is the primary operational mode. It initializes:
   - Configuration loading from `~/.arawn/arawn.toml`
   - SQLite store + JSONL message store
   - LLM client (provider-specific)
   - Tool registry (20+ built-in tools)
   - Plugin system (legacy WASM + new-style directory plugins)
   - Skill registry (built-in + plugin skills)
   - MCP server connections
   - Memory system (global + workstream KB)
   - Permission rules
   - Hook system
   - Config file watcher (hot-reload)
   - Plugin file watcher (hot-reload)
   - Workflow engine (cloacina)
   - Axum WebSocket server on `127.0.0.1:PORT`

2. **`arawn tui [--url WS_URL]`** (or no arguments) -- Launches the terminal UI, which connects to the running server via WebSocket.

3. **`arawn <prompt> [--session UUID]`** -- One-shot CLI mode. Connects to the running server, sends the prompt, streams events, prints the final response.

4. **`arawn plugin <subcommand>`** -- Plugin management CLI (install, uninstall, enable, disable, list, marketplace).

5. **`arawn --list-sessions`** -- Lists sessions directly from the store.

### Library: `crates/arawn/src/lib.rs` (published as `arawn_bin`)

Exposes: `ArawnConfig`, `LlmConfig`, `LocalService`, `ChannelModalPrompt`, `PendingModals`.

## Architecture

### Crate Dependency Graph

```
arawn (binary)
├── arawn-core          (domain types)
├── arawn-engine        (engine, tools, permissions, hooks, plugins, skills)
│   ├── arawn-core
│   ├── arawn-llm
│   ├── arawn-memory
│   ├── arawn-storage
│   ├── arawn-embed
│   └── arawn-tool-plugin
├── arawn-llm           (LLM abstraction)
├── arawn-mcp           (MCP integration)
├── arawn-memory        (knowledge base)
├── arawn-service       (service trait + types)
│   └── arawn-core
├── arawn-storage       (persistence)
│   └── arawn-core
├── arawn-tui           (terminal UI)
│   ├── arawn-engine
│   └── arawn-service
└── arawn-workflow      (workflow engine)
```

### Key Abstractions

- **`ArawnService` trait** (`arawn-service`): The contract between UI and backend. Defines `list_workstreams`, `create_workstream`, `list_sessions`, `create_session`, `load_session`, `send_message`, `cancel`. `LocalService` is the only implementation.

- **`LlmClient` trait** (`arawn-llm`): Provider-agnostic streaming LLM interface. Single method: `stream(ChatRequest) -> Stream<ChatChunk>`. Implementations: `AnthropicClient`, `GroqClient`, `OpenAICompatibleClient`, `MockLlmClient`, `RetryClient` (decorator).

- **`Tool` trait** (`arawn-engine`): Tools the LLM can invoke. Methods: `name()`, `description()`, `parameters_schema()`, `execute(ctx, params)`, `is_read_only()`. Registered in a `ToolRegistry` (concurrent `HashMap` behind `RwLock`).

- **`ToolContext`**: Immutable execution context per session -- working directory, session ID, workstream name, allowed paths, LLM client for sub-queries, agent nesting depth, read-file tracker.

- **`QueryEngine`**: The agentic loop. Owns references to LLM, tool registry, compactor, permission checker, hook runner, skill/plugin registries, plan state, and background task manager. Runs iteratively: prompt LLM -> if tool calls, execute them -> feed results back -> loop.

- **`Session`** (`arawn-core`): A conversation with messages (User, Assistant, ToolResult, Summary). Supports compaction and microcompaction. Tracks `SessionStats` (tokens, turns, tool calls).

- **`Workstream`** (`arawn-core`): An organizational unit with name, root directory, and ID. A "scratch" workstream is always created for ad-hoc sessions.

- **`Store`** (`arawn-storage`): Unified persistence composing SQLite metadata (sessions, workstreams) + JSONL message files (one file per session per workstream directory).

### Data Flow

```
User Input
    │
    ▼
TUI/CLI ──WebSocket──► Server (ws_server.rs)
                          │
                          ▼
                    LocalService.send_message()
                          │
                          ├── Load session from Store (SQLite meta + JSONL messages)
                          ├── Build ToolContext (working dir, allowed paths, LLM)
                          ├── Build PromptContext (OS, shell, context files, memories)
                          ├── Create QueryEngine with compactor, permissions, hooks
                          │
                          ▼
                    QueryEngine.run() [agentic loop]
                          │
                          ├── Drain background task notifications
                          ├── Microcompact old tool results
                          ├── Check compaction threshold → LLM summarization if needed
                          ├── Build system prompt (identity, rules, context files, memories)
                          ├── Stream LLM request with retry
                          ├── If no tool calls → return final text
                          ├── Validate tool calls (reject hallucinated/invalid)
                          ├── Permission check (allow/deny/ask via modal prompt)
                          ├── Execute tools (read-only in parallel, writes serial)
                          ├── Pre/Post tool hooks
                          ├── Limit large results (truncate + persist overflow)
                          ├── Track failed calls for duplicate detection
                          ├── Append messages to session
                          └── Loop back to LLM
                          │
                          ▼
                    Persist new messages (JSONL append)
                    Persist session stats (SQLite)
                    Emit EngineEvents → WebSocket → TUI/CLI
```

## Primary Workflows

### 1. Message Processing (send_message)

**Entry**: `ws_server.rs` receives JSON-RPC `send_message` with `session_id` and `content`.

**LocalService.send_message()** (`local_service.rs:548-820`):
1. Loads session metadata from SQLite, resolves workstream
2. Loads JSONL messages, applies `load_compacted()` (skips messages before last Summary)
3. Reconstructs `Session` from parts
4. Appends user message and persists to JSONL
5. Resolves sandbox directory (per-session for scratch, per-workstream otherwise)
6. Builds `ToolContext` with allowed paths for arawn.md files
7. Builds per-session `PromptContext` with memory injection (L1 wake-up + L2 topical context from user keywords)
8. Creates `QueryEngine` with compactor, permissions, hooks, skills, plugins, plan state, background tasks
9. Spawns async task that:
   - Creates progress channel for live tool call streaming
   - Runs `engine.run(&mut session, &ctx)`
   - Persists all new messages to JSONL
   - Updates session stats in SQLite
   - Emits `Complete` or `Error` event
10. Returns `ReceiverStream` of `EngineEvent` to the WebSocket handler

**QueryEngine.run()** (`query_engine.rs:219-520+`):
- Iterates up to `max_iterations` (default 20)
- Each iteration: microcompact, check compaction, build+stream LLM request, process response
- Tool execution: validates names, checks permissions, fires hooks, executes (read-only parallel, write serial), limits results, tracks failures
- Compaction has a circuit breaker (3 consecutive failures disables it)
- Repeated identical failing tool calls are blocked after 2 failures

### 2. Session/Workstream Management

- **Workstreams**: Created via RPC or agent tools. Stored in SQLite `workstreams` table. Each workstream gets a filesystem directory under `~/.arawn/workstreams/<name>/`.
- **Sessions**: Created per-workstream or as scratch. Metadata in SQLite `sessions` table. Messages in JSONL files at `workstreams/<dir>/sessions/<session_id>.jsonl`.
- **Scratch sessions**: Belong to the special "scratch" workstream. Get per-session sandbox directories.
- **Session promotion**: Scratch sessions can be promoted to a named workstream via `promote_session`.

### 3. Plugin Loading

**Legacy plugins** (WASM via fidius): `.arawn_tool` shared libraries in `~/.arawn/plugins/tools/`, loaded by `PluginLoader`, adapted via `PluginToolAdapter`.

**New-style plugins** (Claude Code compatible): Directories with `plugin.json` manifest under `~/.arawn/plugins/`. `PluginRuntime` discovers and loads plugins, extracting:
- Tools (registered in ToolRegistry)
- Skills (registered in SkillRegistry)
- Hooks (merged into HookConfig)
- MCP server definitions (connected by McpManager)
- Agent definitions

Both plugin types support hot-reload via file watchers.

### 4. Permission Checking

Flow in `query_engine.rs` tool execution:
1. `PermissionChecker` evaluates rules against tool call (name + optional content pattern)
2. Rules are ordered: first match wins. Rule kinds: `Allow`, `Deny`, `Ask`
3. If no rule matches, `PermissionMode` fallback:
   - `Default`: read-only auto-allowed, writes/shell trigger Ask
   - `AcceptEdits`: files auto-allowed, shell triggers Ask
   - `BypassPermissions`: everything auto-allowed
   - `Plan`: only read-only allowed, everything else denied
4. `Ask` decisions send `UserInputRequest` event through WebSocket, pause engine, wait for `user_input_response` from client

### 5. Hook Execution

25 hook event types matching Claude Code's surface area. Hooks are shell commands configured in `settings.json`. The `HookRunner` matches events against configured hooks and executes them. Key events: `PreToolUse` (can block), `PostToolUse`, `Stop`, `SessionStart`, `SessionEnd`, `PreCompact`, `PostCompact`.

### 6. Context Compaction

Two-tier compaction:
- **Microcompaction** (no LLM call): Clears old tool result content for targeted tools (file_read, grep, glob, shell, web_fetch, web_search) when results exceed a size threshold, keeping recent messages intact.
- **Full compaction** (LLM summarization): When estimated token usage exceeds `compaction_threshold` (default 85%) of context window, the `Compactor` sends old messages to the LLM for summarization, replaces them with a `Summary` message, keeps `keep_recent` (default 6) messages verbatim.

## Public Interface Surface

### WebSocket RPC API (JSON-RPC style)

Endpoint: `ws://127.0.0.1:3100/ws`

Methods:
- `list_workstreams` -- Returns `Vec<WorkstreamInfo>`
- `create_workstream` -- Params: `name`, optional `root_dir`
- `list_sessions` -- Params: optional `workstream_id`
- `create_session` -- Params: optional `workstream_id`
- `load_session` -- Params: `session_id`
- `send_message` -- Params: `session_id`, `content` -- Returns streaming `EngineEvent`s
- `user_input_response` -- Params: `request_id`, `selected_index` (for permission modals)
- `query_inventory` -- Params: `kind` (skills, commands, etc.)
- `list_commands` -- Returns available slash commands
- `remember` -- Params: `content` (store fact in KB)
- `memory_summary` -- Returns KB summary
- `forget` -- Params: `query` (delete entity from KB)
- `promote_session` -- Params: `session_id`, `workstream_name`

### HTTP Endpoint

- `POST /api/decision` -- Workflow decision tasks (cloacina pipeline agent execution)

### Streaming Events (`EngineEvent`)

- `StreamingText { text }` -- Chunk of assistant output
- `ToolCallStart { id, name, input }` -- Tool invocation beginning
- `ToolCallResult { id, content, is_error }` -- Tool completion
- `Complete { final_text }` -- Turn finished
- `Error { message }` -- Error during turn
- `CompactionOccurred { messages_summarized }` -- Context was compacted
- `Usage { input_tokens, output_tokens }` -- Token usage
- `UserInputRequest { request_id, title, subtitle, options }` -- Permission prompt
- `Flush` -- Render boundary signal

### Configuration (`~/.arawn/arawn.toml`)

Sections:
- `[llm.<name>]` -- Named LLM configs: provider, model, api_key_env, base_url, context_window, max_tokens
- `[engine]` -- llm reference, max_iterations (20), max_result_size (50000)
- `[compactor]` -- Optional separate LLM, compaction_threshold (0.85), keep_recent (6)
- `[server]` -- host (127.0.0.1), port (3100)
- `[storage]` -- data_dir (~/.arawn)
- `[prompts]` -- token_budget (6000)
- `[sandbox]` -- network_tools list (tools allowed network access in shell sandbox)

Environment variables: `GROQ_API_KEY`, `GROQ_MODEL`, `ARAWN_DATA_DIR`

### CLI

```
arawn serve [--port PORT]
arawn tui [--url WS_URL]
arawn <prompt> [--session UUID]
arawn --list-sessions
arawn plugin install|uninstall|enable|disable|list|marketplace
```

## Dependency Graph

### Key External Dependencies

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime (full features) |
| `axum` | WebSocket server framework |
| `ratatui` + `crossterm` | Terminal UI rendering |
| `rusqlite` (bundled) | SQLite for metadata + memory stores |
| `refinery` | Database migrations |
| `reqwest` | HTTP client for LLM APIs and web fetch |
| `serde` / `serde_json` / `toml` | Serialization |
| `notify` | Filesystem watching (config, plugins) |
| `globwalk` / `ignore` | File discovery for glob/grep tools |
| `similar` | Diff computation for file_edit |
| `htmd` | HTML-to-markdown conversion (web_fetch) |
| `lru` | Caching (web_fetch) |
| `syntect` + `pulldown-cmark` | Markdown rendering in TUI |
| `fidius` | WASM plugin interface |
| `sandbox-runtime` | Shell command sandboxing (vendored, patched) |
| `cloacina` | Workflow/DAG engine |
| `sqlite-vec` | Vector similarity search in memory system |
| `tracing` | Structured logging |
| `uuid` / `chrono` | IDs and timestamps |

### Internal Crate Dependencies

- `arawn-core`: No internal deps (leaf crate)
- `arawn-llm`: No internal deps (leaf crate)
- `arawn-service`: depends on `arawn-core`
- `arawn-storage`: depends on `arawn-core`
- `arawn-tool-plugin`: No internal deps (re-exports `fidius`)
- `arawn-embed`: No internal deps
- `arawn-memory`: No internal deps
- `arawn-mcp`: No internal deps listed in workspace
- `arawn-engine`: depends on `arawn-core`, `arawn-llm`, `arawn-memory`, `arawn-storage`, `arawn-embed`, `arawn-tool-plugin`
- `arawn-tui`: depends on `arawn-engine`, `arawn-service`
- `arawn-workflow`: No workspace deps (uses cloacina)
- `arawn` (binary): depends on all of the above

## Build and Deployment

### Build System

- **Angreal** task runner with Python tasks in `.angreal/`:
  - `angreal build workspace [--release]` -- Cargo build
  - `angreal test all` / `angreal test unit` -- `cargo test --workspace -- --test-threads=1`
  - `angreal test integration` -- `cargo test --workspace -- --ignored --test-threads=1`
  - `angreal test coverage` -- Per-crate branch coverage with `cargo-llvm-cov` (nightly)
- **Flox** environment (`.flox/`) provides reproducible dev shell (Nix-based)
- Tests run single-threaded (`--test-threads=1`) -- likely due to shared SQLite state or port conflicts

### Test Infrastructure

- **Unit tests**: Inline `#[cfg(test)] mod tests` in most source files
- **Integration tests**: `crates/arawn-tests/tests/` with 13 test files covering:
  - Full pipeline (all subsystems wired together)
  - Compaction
  - Engine persistence
  - Hooks
  - Hot reload
  - Local service
  - Memory stack
  - Permissions
  - Plugin components + loading
  - Skills
  - WebSocket integration
  - Workflows
- **Test harness**: `arawn_engine::testing::TestHarness` -- builder pattern for assembling engine test fixtures with `MockLlmClient`
- **Snapshot tests**: TUI rendering tests using `insta`
- **Plugin test fixtures**: Two fixture plugins in `arawn-tests/fixtures/` (web-fetch, web-search)

### Database

- **SQLite** via `rusqlite` (bundled) with WAL mode and 5s busy timeout
- **Migrations**: Refinery-managed in `crates/arawn-storage/migrations/`
  - V1: `workstreams` and `sessions` tables
- **Memory databases**: Separate SQLite DBs per scope (`memory.db` global, `workstreams/<dir>/memory.db` per workstream)

### CI

- GitHub Actions (`.github/workflows/`)

## Conventions and Implicit Knowledge

### Architectural Patterns

- **Builder pattern**: Used extensively -- `QueryEngine`, `ToolContext`, `TestHarness`, `LocalService` all use `with_*` builder methods
- **Trait-based abstractions**: `LlmClient`, `Tool`, `ArawnService`, `ModalPrompt`, `Embedder` -- all use `async_trait`
- **Arc + RwLock/Mutex sharing**: Shared state (tool registry, permission rules, stores) wrapped in `Arc<RwLock<T>>` or `Arc<Mutex<T>>`
- **Channel-based event streaming**: `mpsc` channels for engine events, progress events, modal prompt responses
- **Hot-reload**: Config, plugins (legacy + new), permissions, and MCP servers all support live reloading via file watchers

### Naming Conventions

- Crate names: `arawn-<subsystem>` (hyphenated in Cargo, `arawn_<subsystem>` in Rust code)
- Binary crate's lib is published as `arawn_bin` to avoid name collision
- Tool names: PascalCase in code (`FileReadTool`), snake_case or PascalCase for LLM-facing names (`file_read` or `Read`)
- Workstream directories: `<name>-<uuid>` format via `workstream_dir_name()`

### Error Handling

- `thiserror` for typed errors in each crate (`EngineError`, `StorageError`, `LlmError`, `MemoryError`, `ServiceError`, `EmbedError`)
- `anyhow` at the binary level for ad-hoc errors
- Engine errors propagate to `EngineEvent::Error` for client display

### Message Persistence

- Session metadata (id, workstream_id, created_at, stats) in SQLite
- Message content in JSONL files (one line per message, append-only)
- On load: all JSONL messages loaded, then `load_compacted()` trims before last Summary
- Session reconciliation: cleans up SQLite entries whose JSONL files were deleted

### Sandbox Model

- Shell tool runs commands through `sandbox-runtime` (vendored/patched)
- Network access controlled per-command: if command invokes a tool in `network_tools` list, network is allowed; otherwise blocked
- File tools enforce working directory sandbox; `allowed_paths` list permits access to specific files outside sandbox (e.g., arawn.md)
- File edit/write tools check `read_files` tracker to ensure files are read before modification

### Memory System

- Two-tier: global KB (`~/.arawn/memory.db`) + workstream KB (`workstreams/<dir>/memory.db`)
- Graph-backed entity storage with typed relations, confidence scoring, tags
- FTS5 full-text search + sqlite-vec vector similarity
- `MemoryStack` provides layered injection: L1 wake-up context (always injected), L2 topical context (keyword-matched from user message)
- Entities have types (e.g., Person, Convention, Decision) with default scopes

## Open Questions

1. **Cancellation**: `cancel()` in `LocalService` is a no-op (`TODO: implement cancellation via CancellationToken`). Long-running engine loops cannot be interrupted by the client.

2. **Concurrency model for send_message**: Each `send_message` call spawns a new async task with its own `QueryEngine`. There appears to be no guard against concurrent sends to the same session, which could cause JSONL message interleaving.

3. **Plugin system duality**: Two plugin systems coexist -- legacy WASM (fidius-based `.arawn_tool` plugins) and new-style directory plugins (Claude Code compatible with `plugin.json`). The relationship and migration path between them is not documented in code.

4. **Workstream directory naming**: `workstream_dir_name()` creates directory names from workstream name + UUID. It is unclear what happens if a workstream is renamed.

5. **Single-threaded test requirement**: All tests run with `--test-threads=1`. The reason (likely shared state or port conflicts) is not documented.

6. **TUI dependency on arawn-engine**: The TUI crate depends directly on `arawn-engine` (for `PermissionMode`, `HookConfig` etc.) despite being a WebSocket client. This creates a heavier dependency chain than the client-server architecture would suggest.

7. **Server-required architecture**: Both CLI and TUI modes require a running server. There is no embedded/in-process mode for single-user local operation without starting a separate server process.

8. **Memory/embedding initialization**: The memory system with vector storage is initialized at server startup with hardcoded 384 dimensions. The `arawn-embed` crate supports API-based and local ONNX embedders, but the startup code in `main.rs` uses `try_open_memory` which appears to only initialize the store without an actual embedder.

9. **WebSocket connection handling**: The WS handler processes one connection at a time in a loop. During `send_message` streaming, it uses `tokio::select!` to handle both engine events and client messages (for modal responses), but the pattern for handling multiple concurrent requests on a single connection is not obvious.

10. **Build dependency**: The binary crate has a build dependency on `cloacina-build` for workflow scaffold generation, suggesting compile-time code generation for workflow types.
