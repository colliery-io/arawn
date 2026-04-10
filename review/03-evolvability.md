# Evolvability Review

**Lens**: Can this system be changed safely and confidently?

## Summary

The arawn codebase demonstrates generally strong modular decomposition for a project of its size. The 13-crate workspace follows a clear layered architecture with leaf crates (`arawn-core`, `arawn-llm`) having zero internal dependencies and the engine sitting in the middle. The `LlmClient` trait is an exemplary abstraction boundary -- clean, minimal, and provider-agnostic. The `Tool` trait is well-designed and adding new tools is a low-friction operation.

However, `arawn-engine` has grown into a ~21,000-line mega-crate that houses tools, permissions, hooks, plugins, skills, compaction, planning, and the query loop itself. This accumulation of concerns creates coupling that makes some changes -- particularly to the permission model and plugin system -- harder than they should be. Two upward dependency violations (`arawn-mcp` depends on `arawn-engine`, `arawn-tui` depends on `arawn-engine`) leak engine internals into what should be clean boundary crates. The WebSocket RPC dispatch is a hand-rolled match block that requires coordinated changes across multiple files for each new method. Test architecture is solid but the `--test-threads=1` requirement signals hidden shared-state coupling.

## Architecture Assessment

### Modularity

**Crate count (13) is appropriate** for the current scope. The leaf crates are clean: `arawn-core` (domain types), `arawn-llm` (LLM abstraction), `arawn-embed` (embeddings), and `arawn-tool-plugin` (WASM interface) have no internal dependencies and well-defined single responsibilities.

**`arawn-engine` is too large.** At ~21,000 lines across 60+ files, it houses at least six distinct subsystems:
- Query engine loop (~1,200 lines)
- Tools (22 tools, ~6,900 lines)
- Permissions (~700 lines across 5 files)
- Hooks (~800 lines across 7 files)
- Plugins (~3,200 lines across 9 files)
- Skills (~770 lines across 3 files)
- Supporting infrastructure (compactor, system prompt, token estimator, testing harness, etc.)

Each subsystem has its own internal module structure, but they all live under one crate, meaning any change to any subsystem forces a full recompile of the engine and everything that depends on it. The `lib.rs` re-exports 50+ public symbols in a flat namespace.

**`arawn-memory` and `arawn-storage` are well-scoped.** They each own their persistence domain cleanly. The separation between SQLite-backed session metadata (`arawn-storage`) and SQLite-backed knowledge base (`arawn-memory`) is appropriate since they serve different purposes and have different schemas.

### Coupling

**Good: Engine-LLM boundary.** The engine depends on `arawn-llm` only through the `LlmClient` trait and the shared types (`ChatRequest`, `ChatChunk`, etc.). This is clean and allows provider swaps without touching engine code.

**Good: Engine-Storage boundary.** `arawn-engine` depends on `arawn-storage` but the coupling surface is narrow -- the engine takes sessions as domain objects and doesn't drive SQL directly.

**Problem: `arawn-mcp` depends on `arawn-engine`.** The MCP adapter implements `Tool` and uses `ToolRegistry`, `ToolContext`, and `EngineError` from the engine. This means MCP is coupled to engine internals. If the `Tool` trait or `ToolContext` changed, MCP would break. The `Tool` trait and `ToolRegistry` should arguably live in a separate interface crate that both engine and MCP depend on.

**Problem: `arawn-tui` depends on `arawn-engine`.** The TUI is a WebSocket client, yet it imports `arawn_engine::permissions::ModalPrompt` (and related types). This is used for the `TuiModalPrompt` implementation. The dependency chain means the TUI crate pulls in all of `arawn-engine`'s dependencies (rusqlite, sandbox-runtime, fidius-host, etc.) despite never using them. The types TUI needs (`ModalPrompt`, `PermissionMode`, `HookConfig`) should be in `arawn-service` or a shared types crate.

**Problem: Permission checker uses string-matching for tool categories.** The `tool_category()` function in `permissions/checker.rs` hard-codes tool names to categories. Adding a new tool requires updating this function, which is in a different module than the tool itself. The `Tool` trait already has `is_read_only()`, but the permission system doesn't fully use it -- it has its own parallel categorization.

**Problem: Tool registration is manual and scattered.** Adding a tool requires: (1) creating the tool struct, (2) adding it to `tools/mod.rs` re-exports, (3) adding it to `lib.rs` re-exports, (4) registering it in `main.rs` with `registry.register()`, and (5) potentially updating `tool_category()` in the permission checker. This is five coordinated touch points.

### Cohesion

**Good: Individual tools are self-contained.** Each tool file encapsulates its schema, description, and execution logic. Tools range from 92 lines (ThinkTool) to 844 lines (ShellTool), which is reasonable.

**Good: Hooks subsystem is internally cohesive.** Config, events, executor, matcher, runner, and file watcher form a complete, self-contained subsystem that happens to live inside `arawn-engine`.

**Mixed: Plugin system has two parallel implementations.** Legacy WASM plugins (`plugin_loader.rs`, `plugin_adapter.rs`, `plugin_watcher.rs`) and new-style directory plugins (`plugins/` module with 9 files) coexist. They share the `ToolRegistry` for registration but otherwise duplicate concepts like loading, discovery, and hot-reload.

**Problem: `LocalService` in the binary crate is doing too much.** At 856 lines, it bridges engine, storage, memory, permissions, hooks, skills, plugins, plan state, and background tasks. It assembles `QueryEngine` with ~10 `with_*` calls per message, re-resolving the same configuration each time.

### Abstraction Boundaries

**`LlmClient` trait -- excellent.** Single method (`stream`), fully provider-agnostic, clean types. The `RetryClient` decorator pattern is well-applied. Adding a new provider means implementing one method and adding a constructor. No engine changes required.

**`Tool` trait -- good with friction.** The trait itself is clean: `name()`, `description()`, `parameters_schema()`, `execute()`, `is_read_only()`. But the `execute` method takes `ToolContext` by reference, and `ToolContext` carries an LLM client, model limits, data directory, agent depth, and read-file tracker. This is a "God context" that gives every tool access to capabilities it may not need.

**`ArawnService` trait -- underused.** Defined in `arawn-service` with the comment "Future: `RemoteService`", but only `LocalService` exists. The trait defines 6 methods but the WebSocket server dispatches 12+ RPC methods. The extra methods (`promote_session`, `remember`, `forget`, `memory_summary`, `query_inventory`, `list_commands`, `user_input_response`) bypass the trait entirely and call `LocalService` directly, defeating the abstraction.

**`Store` -- well-bounded.** The composition of SQLite metadata + JSONL message files behind a unified interface is clean. The internal split (`Database`, `SessionStore`, `WorkstreamStore`, `JsonlMessageStore`) is well-factored.

### Dependency Management

**Workspace dependencies are well-managed.** Common crates (`tokio`, `serde`, `uuid`, etc.) use workspace-level version pins. Internal crates are also workspace dependencies (though inconsistently -- `arawn-memory`, `arawn-embed`, `arawn-mcp`, `arawn-tui`, `arawn-workflow` use path dependencies directly instead of workspace references).

**Two separate SQLite compilations.** Both `arawn-storage` and `arawn-memory` depend on `rusqlite` with `features = ["bundled"]`. Since they're separate crates, Cargo deduplicates the dependency, but the fact that two crates independently own SQLite databases means schema management is split. `arawn-storage` uses Refinery migrations; `arawn-memory` appears to create tables programmatically.

**Heavy leaf crate: `arawn-embed`.** This crate pulls in `ort` (ONNX Runtime with download-binaries) and `tokenizers`, which are large native dependencies. Any crate depending on `arawn-embed` inherits significant compile-time cost. Currently only `arawn-engine` depends on it.

**Vendored dependency risk.** `sandbox-runtime` is vendored and patched. This freezes it against upstream changes and means the project must manually maintain the patch.

### Test Architecture

**Integration test coverage is broad.** 13 test files in `arawn-tests` covering the full pipeline, compaction, hooks, hot-reload, permissions, plugins, skills, WebSocket integration, and workflows. The `TestHarness` builder in `arawn-engine/src/testing.rs` (1,924 lines) provides a comprehensive fixture system using `MockLlmClient`.

**Tests support refactoring with caveats.** The `MockLlmClient` scripts responses, letting tests verify engine behavior without real API calls. Tests check behavior (final text, tool calls made, session state) rather than implementation details. However, the mock works by scripting exact response sequences, which means tests are somewhat coupled to the order of engine operations.

**`--test-threads=1` is a red flag.** All tests run single-threaded. This suggests either: shared SQLite state across tests (the `Store` uses file-based databases), port conflicts (WebSocket tests), or both. This limits test parallelism and signals that test isolation is incomplete. As the test suite grows, this will become a progressively worse bottleneck.

**Inline tests are well-placed.** Following the user's preference, test modules live alongside their source. The `tool.rs` tests cover registry operations, concurrency, and prefix-based unregistration thoroughly.

## Change Cost Analysis

### 1. Add a new tool to the engine

**Cost: Low-Medium (5 touch points)**

1. Create `crates/arawn-engine/src/tools/new_tool.rs` implementing the `Tool` trait (~100-400 lines depending on complexity)
2. Add `pub mod new_tool;` and `pub use new_tool::NewTool;` to `tools/mod.rs`
3. Add `NewTool` to the re-export list in `lib.rs`
4. Add `registry.register(Box::new(arawn_engine::NewTool))` in `main.rs` serve mode
5. If write tool: update `tool_category()` in `permissions/checker.rs`

The tool itself is straightforward -- the `Tool` trait is simple and `ToolContext` provides everything needed. The friction is in the registration ceremony and the permission system's string-based categorization.

### 2. Add a new LLM provider

**Cost: Low (2-3 touch points)**

1. Create `crates/arawn-llm/src/new_provider.rs` implementing `LlmClient` (~200-400 lines, mostly SSE parsing)
2. Add `pub mod new_provider;` and re-export in `lib.rs`
3. Add a match arm in `main.rs`'s `build_llm_client()` function

This is the cleanest change path in the system. The `LlmClient` trait is minimal and the provider-neutral types (`ChatRequest`, `ChatChunk`) handle all the abstraction. The existing `AnthropicClient` and `GroqClient` serve as clear templates.

### 3. Change the session storage format

**Cost: Medium-High**

Session data is split across two stores:
- **SQLite** (`sessions` table): metadata, stats, workstream_id -- managed by `SessionStore` with Refinery migrations
- **JSONL files**: message content, one file per session -- managed by `JsonlMessageStore`

Changing metadata format: Add a Refinery migration in `arawn-storage/migrations/`, update `SessionMeta` struct and its SQL queries. The migration infrastructure handles schema evolution.

Changing message format: This is harder. JSONL files are append-only with one `Message` enum variant per line. Changing the `Message` enum in `arawn-core` would break deserialization of existing files. There is no migration mechanism for JSONL files -- `load_compacted()` assumes the current serialization format. A format change would require either: (a) maintaining backward-compatible deserialization with `#[serde(untagged)]` or version tags, (b) writing a one-time migration tool, or (c) accepting that old sessions become unreadable.

The `Session::load_compacted()` optimization (skip messages before last Summary) adds complexity -- any migration must account for partially-loaded sessions.

### 4. Add a new WebSocket RPC method

**Cost: Medium (3-4 touch points, high boilerplate)**

1. Add the method to `ArawnService` trait in `arawn-service` (if it's a core operation) -- or skip this and call `LocalService` directly (as several methods already do)
2. Implement the method in `LocalService` in the binary crate
3. Add a match arm in `ws_server.rs`'s `handle_connection()` -- this means copying ~20 lines of boilerplate (param extraction, service call, response wrapping, send-and-check-error)
4. Update the TUI client in `ws_client.rs` if it needs to call the new method

The WebSocket dispatch is a ~600-line hand-rolled `match` block where each arm repeats the same pattern: extract params from JSON, call service, wrap result, send response, check for send error. There is no code generation, macro, or router abstraction. Each new method adds ~20-30 lines of nearly identical code. This is the highest-boilerplate change in the system.

### 5. Modify the permission system

**Cost: High (wide blast radius)**

The permission system touches many layers:
- `PermissionRule`, `PermissionChecker`, `PermissionMode` (engine crate, `permissions/` module)
- `tool_category()` hard-coded tool name mapping (engine crate)
- `ModalPrompt` trait + implementations (`ChannelModalPrompt` in binary, `TuiModalPrompt` in TUI)
- `PermissionConfig` loading from `settings.json` (engine crate)
- Permission checking in `query_engine.rs` (engine crate)
- `UserInputRequest`/`user_input_response` flow across WebSocket (binary crate)
- `ArawnService` doesn't model permissions, so changes bypass the service abstraction

Changing the permission model (e.g., adding file-path-based rules, or per-workstream permissions) would require coordinated changes across the engine, binary, and TUI crates. The `PermissionMode` enum appears in the TUI despite the TUI being a WebSocket client -- this type should not cross the WebSocket boundary as a Rust type.

## Findings

### EVO-01: `arawn-engine` is a monolithic mega-crate [Medium]

**Location**: `crates/arawn-engine/` (~21,000 lines, 60+ files, 6+ subsystems)

The engine crate houses the query loop, all 22 built-in tools, the permission system, hook system, plugin framework, skill framework, compaction, system prompt building, token estimation, plan mode, background tasks, agent definitions, and the 1,924-line test harness. This violates the single-responsibility principle at the crate level.

**Impact**: Any change to any subsystem triggers full recompilation of the engine and all downstream crates. The flat re-export of 50+ symbols in `lib.rs` creates a wide public API surface that's hard to evolve. Subsystems that could be independently versioned and tested (hooks, permissions, skills) are forced into lockstep.

**Recommendation**: Extract at minimum:
- `arawn-tool` crate: `Tool` trait, `ToolOutput`, `ToolRegistry`, `ToolContext` -- this is what `arawn-mcp` actually needs
- Consider extracting hooks and permissions if they stabilize as independent subsystems

### EVO-02: Upward dependency from `arawn-mcp` to `arawn-engine` [Medium]

**Location**: `crates/arawn-mcp/Cargo.toml` -- depends on `arawn-engine`

The MCP crate imports `Tool`, `ToolOutput`, `ToolRegistry`, `ToolContext`, and `EngineError` from the engine. This creates a circular-ish dependency structure: the binary crate depends on both engine and MCP, and MCP depends back on engine. If the `Tool` trait interface changes, MCP must change in lockstep.

**Impact**: Cannot evolve the engine's internal tool infrastructure without also updating MCP. Compile times are inflated because MCP pulls in the entire engine dependency tree.

**Recommendation**: Extract a `arawn-tool` interface crate containing the `Tool` trait, `ToolOutput`, `ToolRegistry`, `ToolContext`, and `EngineError`. Both `arawn-engine` and `arawn-mcp` would depend on this thin interface crate instead.

### EVO-03: TUI depends on engine internals despite being a WebSocket client [Low]

**Location**: `crates/arawn-tui/Cargo.toml` -- depends on `arawn-engine`; `crates/arawn-tui/src/tui_prompt.rs` imports `arawn_engine::permissions::{ModalPrompt, ModalRequest}`

The TUI is architecturally a thin WebSocket client, but it imports `ModalPrompt` and `ModalRequest` from `arawn-engine`. This pulls in the full engine dependency graph (rusqlite, sandbox-runtime, fidius-host, ort, etc.) for the TUI binary.

**Impact**: TUI compile times are inflated. The types `ModalPrompt` and `ModalRequest` are UI-facing contracts that belong in the service layer, not the engine.

**Recommendation**: Move `ModalPrompt`, `ModalRequest`, `ModalOption`, and `PermissionMode` to `arawn-service`. The TUI already depends on `arawn-service`. This would let the TUI drop its `arawn-engine` dependency entirely.

### EVO-04: WebSocket RPC dispatch is a hand-rolled match block with high boilerplate [Low]

**Location**: `crates/arawn/src/ws_server.rs` (~600 lines of dispatch)

Each RPC method is a match arm that repeats the same pattern: extract params, call service, serialize result, send response, handle send failure. Adding a new method means copying ~25 lines of boilerplate. There are already 12+ methods, and the pattern is identical across all of them.

**Impact**: Adding new RPC methods is tedious and error-prone. The boilerplate makes it easy to introduce inconsistencies (e.g., forgetting error handling, wrong error code).

**Recommendation**: Consider a macro or dispatch table to reduce per-method boilerplate. Even a simple helper function `dispatch_rpc(service, method, params) -> Response` that handles the common send/error pattern would reduce duplication.

### EVO-05: `ArawnService` trait is incomplete -- many RPC methods bypass it [Low]

**Location**: `crates/arawn-service/src/lib.rs` (6 methods) vs `crates/arawn/src/ws_server.rs` (12+ dispatched methods)

The `ArawnService` trait defines `list_workstreams`, `create_workstream`, `list_sessions`, `create_session`, `load_session`, `send_message`, and `cancel`. But the WebSocket server dispatches additional methods (`promote_session`, `remember`, `forget`, `memory_summary`, `query_inventory`, `list_commands`, `user_input_response`) by calling `LocalService` directly, not through the trait.

**Impact**: The trait does not accurately represent the system's capabilities. A hypothetical `RemoteService` implementation would miss half the functionality. The trait comment says "Future: RemoteService" but the architecture has already diverged from that goal.

**Recommendation**: Either expand `ArawnService` to cover all RPC methods, or acknowledge that the trait is not the full contract and document which operations are service-internal.

### EVO-06: Permission system uses parallel categorization, not the `Tool` trait [Low]

**Location**: `crates/arawn-engine/src/permissions/checker.rs`, `tool_category()` function (lines 46-69)

The `Tool` trait has `is_read_only()`, but the permission checker ignores it. Instead, `tool_category()` maps tool names to categories via string matching. This means:
- New tools default to `ToolCategory::Other` (conservative but opaque)
- The categorization can drift from the tool's actual `is_read_only()` declaration
- Plugin tools and MCP tools all fall into `Other` regardless of their nature

**Impact**: Adding a new read-only tool that should be auto-allowed requires updating `tool_category()` in addition to implementing the trait. Plugin tools cannot participate in fine-grained permission categories.

**Recommendation**: Use `tool.is_read_only()` for the read-only/write distinction. Add a `category()` method to the `Tool` trait (with a default that delegates to `is_read_only()`) for finer-grained permission decisions. Retire the string-matching function.

### EVO-07: No JSONL message migration path [Medium]

**Location**: `crates/arawn-storage/src/jsonl.rs`, `crates/arawn-core/src/message.rs`

Session messages are persisted as JSONL files with one serde-serialized `Message` per line. There is no version marker in the file, no migration tooling, and no backward-compatible deserialization strategy. The `Message` enum variants (`User`, `Assistant`, `ToolResult`, `Summary`) are serialized directly.

**Impact**: Any change to the `Message` enum (adding a variant, renaming a field, changing a type) will silently break loading of existing sessions. The `load_compacted()` optimization makes partial failures harder to diagnose. This is a data durability risk as the system accumulates session history.

**Recommendation**: Add a version marker (either as a JSONL header line or a serde tag) to message files. Implement backward-compatible deserialization with `#[serde(alias)]` or explicit version handling. Consider a migration command for format upgrades.

### EVO-08: Dual plugin systems without migration path [Low]

**Location**: Legacy: `plugin_loader.rs`, `plugin_adapter.rs`, `plugin_watcher.rs`; New: `plugins/` module (9 files)

Two complete plugin systems coexist: legacy WASM-based (fidius) and new-style directory plugins (Claude Code compatible). Both have their own discovery, loading, hot-reload, and registration paths. The legacy system is still loaded in `main.rs` and there is no deprecation warning or migration documentation.

**Impact**: Maintenance cost is doubled for plugin infrastructure. Contributors must understand two plugin models. The fidius dependency adds compile-time cost even if no WASM plugins are installed.

**Recommendation**: Establish a deprecation timeline for legacy plugins. Consider feature-gating the fidius dependency so it can be compiled out when not needed.

### EVO-09: `main.rs` contains imperative assembly of 40+ components [Low]

**Location**: `crates/arawn/src/main.rs` (~350 lines of serve mode setup)

The serve mode startup imperatively constructs and wires together the tool registry (20+ tools), plugin systems, MCP connections, memory system, engine config, permission rules, hooks, skill registry, config watcher, and plugin watcher. Each component is constructed with specific initialization ordering and shared-state wiring.

**Impact**: Understanding the system's runtime composition requires reading through a long procedural block. Adding a new subsystem means inserting it into the right place in this sequence. There is no declarative assembly or dependency injection. Testing the startup sequence requires integration tests against the full stack.

**Recommendation**: Consider a builder or registry-based startup that makes component dependencies explicit. The `LocalService` builder pattern is a partial step in this direction but doesn't cover tool registration or plugin loading.
