# Legibility Review

## Summary

Arawn is a well-structured Rust workspace whose crate decomposition genuinely follows its architectural layers: core domain types at the bottom, engine and storage in the middle, and the binary crate at the top. A newcomer can orient themselves quickly because the crate names map clearly to responsibilities and the file layout within each crate is logical. The system-level documentation in `review/00-system-overview.md` is exceptionally thorough and would serve as a genuine onboarding document.

The primary legibility problems are naming inconsistencies (tool names mix snake_case and PascalCase with no discernible rule), a monolithic `main.rs` that is ~650 lines of procedural setup with no organizational abstraction, and a `local_service.rs` whose `send_message` method spans ~220 lines of deeply nested async orchestration. The engine crate also carries two generations of plugin infrastructure (`plugin_loader.rs`/`plugin_adapter.rs`/`plugin_watcher.rs` for the legacy WASM system alongside `plugins/` for the new system) which creates confusion about which system is canonical. Overall, the codebase is readable by Rust standards but has accumulated enough inconsistency and complexity hotspots that a newcomer would stumble in specific, predictable places.

One cross-cutting observation: the code is almost entirely devoid of doc comments on public types and functions outside of a few module-level `//!` headers. The types are often self-explanatory enough, but the absence of `///` docs on key structs like `ToolContext`, `QueryEngineConfig`, `LocalService`, and `Compactor` means a newcomer must read the implementation to understand the contract. This is a systemic gap rather than a localized one.

## Key Themes

1. **Naming inconsistency in tool names**: Tool names visible to the LLM mix `snake_case` and `PascalCase` with no pattern. This extends to the tool filtering constants where the same constant arrays reference both conventions.

2. **Monolithic orchestration functions**: `main.rs` and `LocalService::send_message` are the two primary "wiring" functions and both are very long, procedural, and lack intermediate abstractions. A newcomer can follow them linearly, but modification or debugging requires holding the entire function in mind.

3. **Legacy/new plugin duality**: Two complete plugin systems coexist with no code-level documentation explaining the relationship or migration path. The naming (`plugin_loader` vs `plugins/loader`) adds to the confusion.

4. **Good trait boundaries but heavy builder boilerplate**: The core traits (`Tool`, `LlmClient`, `ArawnService`) are clean and well-chosen. However, the `QueryEngine` and `LocalService` both use extensive `with_*` builder methods for optional components, making construction sites (like `send_message`) verbose.

5. **Private types shadowing public types**: The query engine defines a private `ToolResult` struct that is semantically identical to the public `ToolOutput` but with different field names, creating confusion.

## Findings

## LEG-001: Tool name casing is inconsistent
**Severity**: Major
**Location**: `crates/arawn-engine/src/tools/` (all tool files), `crates/arawn-engine/src/query_engine.rs:869-891`
**Confidence**: High

### Description
Tool names exposed to the LLM use a mix of `snake_case` and `PascalCase` with no discernible rule. Most tools use `snake_case` (`file_read`, `file_edit`, `shell`, `grep`, `web_fetch`, `memory_store`, `task_create`, `task_update`, `task_list`, `task_get`), but several use `PascalCase` (`EnterPlanMode`, `ExitPlanMode`, `Skill`, `TaskOutput`, `TaskStop`). The `agent` tool uses lowercase without underscores.

### Evidence
From `query_engine.rs` tool filter constants:
```rust
const CORE_TOOLS: &[&str] = &["think", "shell", "file_read", "file_write", "file_edit", "glob", "grep", "Skill"];
const PLAN_TOOLS: &[&str] = &["EnterPlanMode", "ExitPlanMode"];
const TASK_TOOLS: &[&str] = &["TaskCreate", "TaskUpdate", "TaskGet", "TaskList", "TaskOutput", "TaskStop"];
const MEMORY_TOOLS: &[&str] = &["memory_store", "memory_search"];
const AGENT_TOOLS: &[&str] = &["Agent"];
```

The `TASK_TOOLS` constant lists `TaskCreate`, `TaskUpdate`, etc. in PascalCase, but the actual tool implementations in `task_list.rs` return `"task_create"`, `"task_update"`, `"task_list"`, `"task_get"` in snake_case. Only `TaskOutput` and `TaskStop` actually use PascalCase. This means the filter will **never match** the snake_case tools because the constant says `"TaskCreate"` but the tool name is `"task_create"`.

### Suggested Resolution
Pick one convention (snake_case is idiomatic for function/command-style names) and apply it universally. Fix the `TASK_TOOLS` constant to match actual tool names. Run a search-and-replace across all tool name definitions.

---

## LEG-002: Private `ToolResult` shadows conceptually identical `ToolOutput`
**Severity**: Minor
**Location**: `crates/arawn-engine/src/query_engine.rs:863-866`, `crates/arawn-engine/src/tool.rs:12-16`
**Confidence**: High

### Description
The query engine defines a private `ToolResult { content: String, is_error: bool }` that is structurally identical to the public `ToolOutput { content: String, is_error: bool }`. The `execute_tool` method returns `ToolResult`, which is then converted into `ToolOutput` for the result limiter, then back into message fields. This creates a confusing naming collision with `arawn_core::Message::ToolResult` (the message variant).

### Evidence
```rust
// query_engine.rs:863
struct ToolResult {
    content: String,
    is_error: bool,
}

// tool.rs:12
pub struct ToolOutput {
    pub content: String,
    pub is_error: bool,
}
```

### Suggested Resolution
Remove the private `ToolResult` and use `ToolOutput` directly in the query engine. This eliminates a needless type and avoids name collision with the `Message::ToolResult` variant.

---

## LEG-003: `main.rs` is a 650-line monolithic startup function
**Severity**: Major
**Location**: `crates/arawn/src/main.rs:27-470`
**Confidence**: High

### Description
The `main()` function handles CLI argument parsing (hand-rolled, not using clap), config loading, tracing initialization, store opening, scratch workstream creation, session listing, serve mode (tool registration, plugin loading, MCP connection, memory init, workflow engine, config watcher), TUI mode, and CLI mode all in a single function. The serve mode block alone is ~220 lines of sequential setup with no intermediate abstractions.

A newcomer looking for "how does tool registration work" or "where is the MCP manager created" must scan linearly through the entire function.

### Evidence
Lines 212-431 comprise the serve mode block, registering 20+ tools individually, loading two plugin systems, connecting MCP servers, building engine config, initializing memory, starting workflow engine, spawning config watcher, and finally starting the server.

### Suggested Resolution
Extract serve mode initialization into a dedicated `fn serve(config, store, port)` or a builder/struct. Group tool registration into a `fn register_default_tools(registry, config, bg_manager)` function. Extract CLI argument parsing into a struct (or adopt clap).

---

## LEG-004: `LocalService::send_message` is ~220 lines of deeply nested async orchestration
**Severity**: Major
**Location**: `crates/arawn/src/local_service.rs:548-820`
**Confidence**: High

### Description
This single method loads session metadata, loads JSONL messages, reconstructs the session, appends the user message, persists it, resolves sandbox directories, builds `ToolContext`, builds `PromptContext` (including L1/L2 memory injection with inline keyword extraction), constructs `QueryEngine` with compactor and all optional components, sets up permission checker, creates progress channels, and spawns the async engine task with its own progress forwarder. The spawned task closure alone is ~100 lines.

### Evidence
The method body starts at line 548 and the closing brace of the `ArawnService` impl is at line 827. The spawned task closure (line 720-817) handles result persistence, stats update, and event emission.

### Suggested Resolution
Extract session loading/reconstruction into a helper. Extract `ToolContext` + `PromptContext` building into a `fn build_session_context(...)`. Extract engine construction into a `fn build_engine(...)`. The spawned task body could be a named async function.

---

## LEG-005: Dual plugin systems without code-level explanation
**Severity**: Minor
**Location**: `crates/arawn-engine/src/plugin_loader.rs`, `crates/arawn-engine/src/plugin_adapter.rs`, `crates/arawn-engine/src/plugin_watcher.rs`, `crates/arawn-engine/src/plugins/`
**Confidence**: High

### Description
Two complete plugin systems coexist: the legacy WASM-based system (via `fidius`, files at `plugin_loader.rs`, `plugin_adapter.rs`, `plugin_watcher.rs`) and the new directory-based system (`plugins/` module). Both are loaded in `main.rs` and both have hot-reload watchers. There is no module-level comment or doc explaining which is preferred, whether the legacy system is deprecated, or when to use one vs the other.

The naming is also confusing: `plugin_loader.rs` is the legacy WASM loader, while `plugins/loader.rs` is the new plugin loader. A newcomer searching for "plugin loading" will find both and not know which is canonical.

### Evidence
In `main.rs`:
```rust
// Load legacy .arawn_tool plugins
let plugin_tools = arawn_engine::PluginLoader::load_tools(&tools_dir, &build_dir);

// Load new-style plugins (Claude Code compatible)
let plugin_runtime = PluginRuntime::new(plugins_root);
```

### Suggested Resolution
Add a deprecation notice to `plugin_loader.rs`, `plugin_adapter.rs`, and `plugin_watcher.rs`. Move them into a `legacy_plugins/` module or add prominent `//! DEPRECATED` module docs. Update the `plugins/mod.rs` doc comment to clarify it is the canonical system.

---

## LEG-006: `task_list.rs` contains 4 unrelated tool implementations
**Severity**: Minor
**Location**: `crates/arawn-engine/src/tools/task_list.rs` (801 lines)
**Confidence**: High

### Description
The file `task_list.rs` defines `SessionTaskStore`, `SessionTask`, `TaskStatus`, and four separate tools: `TaskCreateTool`, `TaskUpdateTool`, `TaskListTool`, and `TaskGetTool`. Meanwhile, `TaskOutputTool` and `TaskStopTool` each have their own files (`task_output.rs`, `task_stop.rs`). The naming suggests `task_list.rs` should only contain the list tool, but it is actually the entire task subsystem.

### Evidence
The file exports from `mod.rs`:
```rust
pub use task_list::{SessionTaskStore, TaskCreateTool, TaskGetTool, TaskListTool, TaskUpdateTool};
pub use task_output::TaskOutputTool;
pub use task_stop::TaskStopTool;
```

### Suggested Resolution
Rename `task_list.rs` to `tasks.rs` (or `task.rs`) to reflect that it contains the task domain model and all CRUD tools. Alternatively, split into `tasks/mod.rs`, `tasks/store.rs`, `tasks/create.rs`, etc. if the 801-line file warrants it.

---

## LEG-007: Stale/duplicate doc comment on `shared_store`
**Severity**: Minor
**Location**: `crates/arawn/src/local_service.rs:80-82`
**Confidence**: High

### Description
The `shared_store` method has two doc comments stacked, the first of which belongs to a different method:

```rust
/// Get a reference to the shared permission rules for hot-reload.
/// Get a shared reference to the store for tools that need direct access.
pub fn shared_store(&self) -> Arc<Mutex<Store>> {
```

The first line is a leftover from `shared_permission_rules` (defined at line 98).

### Suggested Resolution
Remove the stale first doc comment line.

---

## LEG-008: `dirs_path()` is a no-op platform branch
**Severity**: Observation
**Location**: `crates/arawn/src/main.rs:645-654`
**Confidence**: High

### Description
The function has `#[cfg(target_os = "macos")]` and `#[cfg(not(target_os = "macos"))]` branches, but both contain identical code. This suggests either a planned platform distinction that was never implemented, or a refactoring that left dead conditional compilation.

### Evidence
```rust
fn dirs_path() -> Option<String> {
    #[cfg(target_os = "macos")]
    { std::env::var("HOME").ok().map(|h| format!("{h}/.arawn")) }
    #[cfg(not(target_os = "macos"))]
    { std::env::var("HOME").ok().map(|h| format!("{h}/.arawn")) }
}
```

### Suggested Resolution
Remove the platform conditional and keep a single body.

---

## LEG-009: `TASK_TOOLS` filter constant uses wrong casing, tools will never be filtered in
**Severity**: Major
**Location**: `crates/arawn-engine/src/query_engine.rs:880-882`
**Confidence**: High

### Description
The `TASK_TOOLS` constant lists tool names in PascalCase (`"TaskCreate"`, `"TaskUpdate"`, `"TaskGet"`, `"TaskList"`), but the actual tools registered in the `ToolRegistry` use snake_case names (`"task_create"`, `"task_update"`, `"task_get"`, `"task_list"`). Since the filter uses exact string matching (`include.contains(t.name.as_str())`), these four tools will never be included by the task-keyword filter. They can still appear on the first turn (all tools are included for short sessions) and if previously used (the `used_tool_names` check), but the keyword-triggered inclusion path is broken.

### Evidence
From `task_list.rs`:
```rust
fn name(&self) -> &str { "task_create" }  // line 143
fn name(&self) -> &str { "task_update" }  // line 222
fn name(&self) -> &str { "task_list" }    // line 350
fn name(&self) -> &str { "task_get" }     // line 413
```

But in `query_engine.rs`:
```rust
const TASK_TOOLS: &[&str] = &["TaskCreate", "TaskUpdate", "TaskGet", "TaskList", "TaskOutput", "TaskStop"];
```

Only `TaskOutput` (from `task_output.rs`) and `TaskStop` (from `task_stop.rs`) actually use PascalCase and would match.

### Suggested Resolution
Fix the constant to match actual tool names: `["task_create", "task_update", "task_get", "task_list", "TaskOutput", "TaskStop"]`. Better yet, normalize all tool names to one convention (see LEG-001).

---

## LEG-010: `AGENT_TOOLS` constant references `"Agent"` but tool name is `"agent"`
**Severity**: Minor
**Location**: `crates/arawn-engine/src/query_engine.rs:888`, `crates/arawn-engine/src/tools/agent.rs:53`
**Confidence**: High

### Description
Same class of bug as LEG-009. The `AGENT_TOOLS` constant uses `"Agent"` but the `AgentTool::name()` returns `"agent"`. The keyword-triggered inclusion for agent delegation will never match.

### Evidence
```rust
const AGENT_TOOLS: &[&str] = &["Agent"];  // query_engine.rs:888
fn name(&self) -> &str { "agent" }         // agent.rs:53
```

### Suggested Resolution
Fix to `&["agent"]`.

---

## LEG-011: Absence of doc comments on public API types
**Severity**: Minor
**Location**: Systemic across all crates
**Confidence**: High

### Description
Most public structs, enums, and functions lack `///` doc comments. Module-level `//!` docs exist in some places (e.g., `plugins/mod.rs`, `hooks/mod.rs`, `memory/store.rs`) and are helpful, but individual type and method docs are sparse. Key types like `ToolContext`, `QueryEngineConfig`, `PromptContext`, `ToolRegistry`, `Compactor`, `LocalService`, and all tool implementations have no doc comments on their fields or methods beyond occasional inline comments.

### Evidence
- `ToolContext` struct at `context.rs:18` has a struct doc but no field docs except inline comments
- `QueryEngineConfig` at `query_engine.rs:62` has field docs for some fields but not others
- `LocalService` at `local_service.rs:24` has a good struct doc but most `with_*` methods lack docs
- All 20+ tool implementations have `description()` (for the LLM) but no `///` doc comments explaining their Rust API

### Suggested Resolution
Prioritize doc comments on types in `arawn-service` (the public contract), `arawn-engine`'s main orchestration types, and the `Tool` trait methods. Field-level docs on `QueryEngineConfig` and `ToolContext` would be especially valuable since these are the primary configuration surfaces.

---

## LEG-012: WebSocket handler uses manual RPC dispatch with heavy boilerplate
**Severity**: Minor
**Location**: `crates/arawn/src/ws_server.rs:168-700`
**Confidence**: Medium

### Description
The `handle_connection` function is a ~580-line match statement dispatching on `request.method` strings. Each branch follows the same pattern: extract params, call service method, build Response, send over socket, check for send failure. This produces significant visual repetition. The `send_message` branch is particularly complex due to the `tokio::select!` loop for handling engine events alongside incoming modal responses.

### Evidence
Each branch follows:
```rust
"method_name" => {
    let param = request.params.get("key").and_then(...);
    let resp = match service.method(param).await {
        Ok(val) => Response::success(id, serde_json::to_value(&val).unwrap()),
        Err(e) => Response::error(id, "service_error", e.to_string()),
    };
    if sender.send(...).await.is_err() { break; }
}
```

### Suggested Resolution
Extract a macro or helper function for the simple RPC branches. The `send_message` streaming branch is inherently complex and is fine as-is, but the 8+ simple request-response branches could share a pattern.

---

## LEG-013: `ToolOutput` vs `ToolResult` vs `EngineEvent::ToolCallResult` naming confusion
**Severity**: Minor
**Location**: Multiple crates
**Confidence**: Medium

### Description
The word "result" appears in three different contexts with slightly different meanings:
- `ToolOutput` (`arawn-engine/tool.rs`): what a tool returns
- `ToolResult` (`arawn-engine/query_engine.rs`): private struct in the engine loop, same fields as `ToolOutput`
- `Message::ToolResult` (`arawn-core/message.rs`): a message variant for persisted tool results
- `EngineEvent::ToolCallResult` (`arawn-service/types.rs`): a streaming event for tool completion
- `ProgressEvent::ToolCallResult` (`arawn-engine/query_engine.rs`): internal progress event

A newcomer encountering `ToolResult` must determine which of these five things is being referenced.

### Suggested Resolution
Consistent naming: keep `ToolOutput` for the tool return type, `Message::ToolResult` for the persisted message (this is fine as a variant name), and consider renaming `ProgressEvent::ToolCallResult` and `EngineEvent::ToolCallResult` to `ToolCallComplete` or similar to differentiate from the data type.

---

## Cross-Cutting Implications for Other Lenses

- **Correctness**: LEG-009 and LEG-010 are functional bugs -- the tool filtering constants use wrong casing, meaning keyword-triggered tool inclusion is broken for task and agent tools. This affects the LLM's ability to use these tools after the first turn in longer sessions.
- **Performance**: The `filter_tools_for_context` function scans every message in the session history on every turn. For long sessions this grows linearly. May be worth noting for the performance review.
- **Architecture**: The TUI crate depending on `arawn-engine` (for `PermissionMode`, `HookConfig`, etc.) despite being a WebSocket client is an architectural coupling that a dependency/architecture review should flag.
- **Testing**: The casing mismatches in tool filter constants suggest no test covers the "keyword triggers tool inclusion" path for task/agent tools specifically.
