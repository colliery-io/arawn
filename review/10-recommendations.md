# Architecture Recommendations: Arawn

## Overview

Recommendations are grouped by urgency and ordered by impact within each group. Related findings are consolidated into single recommendations where they share a root cause or solution. Effort estimates assume familiarity with the codebase.

**Severity levels referenced:** High findings must be addressed before further development. Medium findings should be addressed in the next development cycle. Low findings are improvements to schedule as structural work.

**Effort scale:** Hours (< 4h), Days (1-3 days), Weeks (1-2 weeks).

## Immediate Actions

These address High severity findings and active bugs. Address before further feature development.

---

### R-01: Fix Tool Name Casing Mismatches in Filter Constants

- **Addresses**: LEG-009, LEG-010, COR-007, LEG-001
- **Severity of addressed findings**: High (adjusted)
- **Effort**: Hours
- **What to do**: Fix `TASK_TOOLS` to use actual tool names (`"task_create"`, `"task_update"`, `"task_get"`, `"task_list"`) and `AGENT_TOOLS` to use `"agent"`. Add workstream tools to a filter category. Normalize all tool names to `snake_case`.
- **Why it matters**: The keyword-based tool filtering silently never matches task or agent tools after the first 2 messages. This degrades LLM capability in longer sessions with no visible error.
- **Suggested approach**: Search all `fn name(&self)` returns across tools. Update the filter constants to match exactly. Rename PascalCase tools (`EnterPlanMode`, `ExitPlanMode`, `Skill`, `TaskOutput`, `TaskStop`) to snake_case. Update any tests or references. Add a test that verifies all tool names in filter constants exist in the registry.
- **Dependencies**: None.

---

### R-02: Add Path Restrictions to Grep and Glob Tools

- **Addresses**: SEC-001
- **Severity of addressed findings**: High
- **Effort**: Hours
- **What to do**: Apply the same `canonicalize + starts_with(root)` check used by `file_read` to the `grep` and `glob` tools. Reject absolute paths and paths that resolve outside the workstream root, with an escape hatch for `ctx.allowed_paths`.
- **Why it matters**: These read-only tools bypass all permission checks and currently allow full-disk search. A prompt injection in any file or web page could instruct the LLM to search sensitive directories.
- **Suggested approach**: Extract the path validation logic from `file_read.rs` into a shared function in `context.rs` or a utility module. Call it from `grep.rs` and `glob.rs` before executing the search. Test with absolute paths, relative traversal paths, and allowed_paths exceptions.
- **Dependencies**: None.

---

### R-03: Implement Sandbox for Background Shell Commands

- **Addresses**: SEC-004
- **Severity of addressed findings**: High
- **Effort**: Days
- **What to do**: Either implement sandbox support for background commands or add a separate permission gate that cannot be bypassed by session grants.
- **Why it matters**: Setting `run_in_background: true` escapes the entire OS-level sandbox. In `AcceptEdits` or `BypassPermissions` mode, background commands execute unsandboxed without user interaction.
- **Suggested approach**: If the sandbox lifecycle cannot support background processes, add a mandatory `Ask` permission check for background execution that is not overridable by session grants or permission mode. Log and surface when background mode is used.
- **Dependencies**: None, but benefits from R-05 (session grant fix).

---

### R-04: Add Per-Session Lock to Prevent Concurrent Access

- **Addresses**: COR-001
- **Severity of addressed findings**: High
- **Effort**: Hours
- **What to do**: Add a per-session lock (e.g., `DashMap<Uuid, Arc<Mutex<()>>>`) in `LocalService`. Acquire before loading session state, release after persisting results. Reject concurrent sends with a clear error.
- **Why it matters**: Two concurrent `send_message` calls to the same session will interleave JSONL writes, corrupting the session history.
- **Suggested approach**: Add a `session_locks: Arc<DashMap<Uuid, Arc<Mutex<()>>>>` field to `LocalService`. In `send_message`, try to acquire the lock. If already held, return `ServiceError::InvalidOperation("Session is currently processing a message")`.
- **Dependencies**: None.

---

### R-05: Fix Session Grants to Respect Deny Rules

- **Addresses**: COR-002, SEC-005
- **Severity of addressed findings**: High (adjusted)
- **Effort**: Hours
- **What to do**: Reorder the permission check in `checker.rs` so deny rules are evaluated before session grants. Priority should be: Deny > SessionGrant > Allow > Ask > mode fallback.
- **Why it matters**: Currently, clicking "Allow Always" for a tool permanently overrides any deny rule for that session. Hot-reloaded deny rules have no effect on already-granted tools, making deny rules an unreliable security boundary.
- **Suggested approach**: In `PermissionChecker::check()`, move the deny-rule evaluation block before the session grant check at line 234. Update the test at line 512 to verify the new behavior. Add a test that deny rules override session grants.
- **Dependencies**: None.

---

### R-06: Surface Persistence Errors to Users

- **Addresses**: OPS-02
- **Severity of addressed findings**: High (adjusted)
- **Effort**: Hours
- **What to do**: When JSONL message persistence fails in the spawned engine task, emit a warning via `EngineEvent` before the `Complete` event. Stop silencing `update_session_stats` errors.
- **Why it matters**: Currently, if disk is full or JSONL write fails, the user sees a normal "Complete" response but their conversation was not saved. This is invisible data loss.
- **Suggested approach**: Replace the `if let Err(e)` pattern with an error accumulator. If any message failed to persist, emit `EngineEvent::Error { message: "Warning: some messages could not be saved to disk. Your conversation may not survive a restart." }` before the `Complete` event. Log `update_session_stats` errors at warn level instead of silencing them.
- **Dependencies**: None.

---

### R-07: Implement Cancellation

- **Addresses**: OPS-10, XC-06
- **Severity of addressed findings**: High (adjusted)
- **Effort**: Days
- **What to do**: Implement `cancel()` using `CancellationToken`. The token should be checked at each iteration of the agentic loop and before each tool execution.
- **Why it matters**: The current implementation returns success but does nothing. The user believes they cancelled, but the engine continues consuming LLM credits and executing tools.
- **Suggested approach**: Add a `CancellationToken` field to `LocalService` keyed by session ID (same `DashMap` as R-04). Pass the token into `QueryEngine`. Check `token.is_cancelled()` at the top of each loop iteration and before each tool execution. On cancellation, emit `EngineEvent::Error { message: "Cancelled by user" }` and break.
- **Dependencies**: Benefits from R-04 (per-session lock infrastructure can share the session map).

## Short-Term Actions

Address in the next development cycle. These are Medium severity findings or groups that improve reliability and developer experience.

---

### R-08: Harden JSONL Persistence

- **Addresses**: COR-008, PERF-03, OPS-06, EVO-07, XC-02
- **Severity of addressed findings**: Medium
- **Effort**: Days
- **What to do**: Add skip-bad-lines recovery in `load()`, record the last-Summary byte offset in SQLite for efficient seeking, and add a version header line for future migration support.
- **Why it matters**: Four findings stem from the same root cause -- JSONL was chosen for simplicity but never gained durability mechanisms. A crash can produce an unrecoverable session, loading parses hundreds of discarded messages, and there is no migration path for format changes.
- **Suggested approach**:
  1. In `load()`, wrap each `serde_json::from_str` in a match and skip lines that fail to parse, logging a warning with the line number.
  2. Add a `last_summary_offset` column to the `sessions` SQLite table. Update it when a Summary message is appended. In `load()`, seek to that offset instead of parsing from the start.
  3. Add a version header as the first line of new JSONL files: `{"_version": 1}`. On load, check for it and branch on version.
  4. Batch multiple message appends within a single turn into one file open/write/close cycle.
- **Dependencies**: None.

---

### R-09: Add WebSocket Authentication

- **Addresses**: SEC-002, SEC-007
- **Severity of addressed findings**: High
- **Effort**: Days
- **What to do**: Generate a session token at server startup, print it to stderr, and require it as a query parameter or first message on WebSocket connections. Require user confirmation for `set_permission_mode` changes to `bypass`.
- **Why it matters**: Any local process or malicious webpage can connect to the WebSocket, call `set_permission_mode(bypass)`, and then issue `send_message` calls that execute arbitrary commands without user interaction.
- **Suggested approach**: Generate a random token at startup, write it to `~/.arawn/server.token`. The TUI reads this file to authenticate. Validate the `Origin` header on WebSocket upgrade to reject browser-initiated connections. For `set_permission_mode(bypass)`, require a modal confirmation prompt. Make the token optional via config for development setups.
- **Dependencies**: None.

---

### R-10: Surface Sandbox Failures and Hook Security

- **Addresses**: SEC-006, SEC-003, SEC-008
- **Severity of addressed findings**: High (SEC-003), Medium (SEC-006, SEC-008)
- **Effort**: Days
- **What to do**: Surface sandbox unavailability as a user-visible warning. Display hook and MCP server commands when installing/enabling plugins. Consider running hooks through the sandbox.
- **Why it matters**: Sandbox failure silently falls back to unsandboxed execution, meaning all sensitive path protections evaporate without the user knowing. Plugins can inject arbitrary hook commands and MCP server processes.
- **Suggested approach**:
  1. When `build_sandbox_config` fails or sandbox is unavailable, emit a warning through the engine event stream (not just a log line). Consider requiring an explicit permission prompt for shell commands when sandbox is unavailable.
  2. When a plugin is loaded, log all hook registrations and MCP server commands at info level. Add a `plugin inspect` CLI command that shows what a plugin will execute.
  3. For hooks, at minimum document that they run unsandboxed. Consider restricting hook working directories to the workstream root.
- **Dependencies**: None.

---

### R-11: Fix Promotion Atomicity

- **Addresses**: COR-004
- **Severity of addressed findings**: Medium
- **Effort**: Hours
- **What to do**: Move the JSONL file before updating SQLite metadata, so failure leaves the session in its original location.
- **Why it matters**: If the file move fails after the SQLite update, the session appears in the workstream list but its JSONL file is still in the scratch directory, making it unloadable.
- **Suggested approach**: Reverse the operation order: copy/move JSONL first, then update SQLite. If the SQLite update fails, move the file back. Add a startup reconciliation check that detects orphaned sessions.
- **Dependencies**: None.

---

### R-12: Log Malformed LLM Arguments

- **Addresses**: COR-003
- **Severity of addressed findings**: Medium
- **Effort**: Hours
- **What to do**: Log a warning when `parse_arguments()` falls back to `{}` due to malformed JSON. Include the raw string (truncated) in the log.
- **Why it matters**: When the LLM produces bad argument JSON, the tool receives empty arguments and fails with a confusing error. The root cause is invisible.
- **Suggested approach**: Add `warn!("Malformed tool arguments from LLM, falling back to empty: {:?}", &raw[..raw.len().min(200)])` in the `unwrap_or` branch. Consider returning a `ToolResult` error directly rather than passing `{}` to the tool.
- **Dependencies**: None.

---

### R-13: Add Signal Handling and Basic Graceful Shutdown

- **Addresses**: OPS-01
- **Severity of addressed findings**: Medium
- **Effort**: Days
- **What to do**: Handle SIGINT/SIGTERM, cancel in-flight engine tasks, and drain background tasks with a timeout.
- **Why it matters**: Without signal handling, process termination kills engine loops mid-execution, potentially leaving partial JSONL writes and orphaned background tasks.
- **Suggested approach**: Use `axum::serve(...).with_graceful_shutdown(shutdown_signal())` where `shutdown_signal()` listens for `tokio::signal::ctrl_c()`. On shutdown: cancel all active engine tasks via the cancellation tokens from R-07, give background tasks 5 seconds to complete, then exit. The workflow runner's `shutdown()` method should run in this window.
- **Dependencies**: R-07 (cancellation implementation provides the mechanism to stop in-flight tasks).

---

### R-14: Fix `truncate_input` UTF-8 Panic

- **Addresses**: COR-005
- **Severity of addressed findings**: Low
- **Effort**: Hours
- **What to do**: Use the char-boundary search pattern from `tool_result_limiter.rs` in `truncate_input()`.
- **Why it matters**: Will panic on non-ASCII tool input content longer than 200 characters (file paths with Unicode, commands with emoji).
- **Suggested approach**: Replace `&input[..max_len]` with `&input[..input.floor_char_boundary(max_len)]` (available in Rust 1.73+) or the manual search from `tool_result_limiter.rs:64-68`.
- **Dependencies**: None.

## Structural Improvements

Larger efforts to schedule. These address Medium/Low findings and systemic patterns.

---

### R-15: Extract `arawn-tool` Interface Crate

- **Addresses**: EVO-01, EVO-02, EVO-03, EVO-06
- **Severity of addressed findings**: Medium
- **Effort**: Weeks
- **What to do**: Extract `Tool`, `ToolOutput`, `ToolRegistry`, `ToolContext`, and `EngineError` into a new `arawn-tool` crate. Move `ModalPrompt`, `ModalRequest`, `ModalOption`, and `PermissionMode` to `arawn-service`.
- **Why it matters**: This is the single highest-leverage structural change. It fixes the upward dependency from `arawn-mcp` into engine internals, lets the TUI drop its engine dependency entirely, and creates a clean boundary for the permission system and future plugin API.
- **Suggested approach**:
  1. Create `crates/arawn-tool/` with the `Tool` trait, `ToolOutput`, `ToolRegistry`, `ToolContext`.
  2. Update `arawn-engine` and `arawn-mcp` to depend on `arawn-tool` instead of the full engine.
  3. Move `ModalPrompt`, `PermissionMode`, and related types to `arawn-service`.
  4. Update `arawn-tui` to depend only on `arawn-service` (drop `arawn-engine`).
  5. Consider adding a `category()` method to the `Tool` trait to replace the string-based `tool_category()` function.
- **Dependencies**: None. This is a refactoring that can happen independently.

---

### R-16: Complete the `ArawnService` Trait

- **Addresses**: API-002, EVO-05, API-003, API-006, XC-04
- **Severity of addressed findings**: Medium
- **Effort**: Days
- **What to do**: Expand `ArawnService` to cover all RPC methods. Define typed response structs for methods that currently return `serde_json::Value`. Map `ServiceError` variants to distinct RPC error codes.
- **Why it matters**: The trait currently covers 7 of 16 RPC methods, making it a misleading abstraction. Half the methods return untyped JSON, and all errors flatten to `"service_error"`.
- **Suggested approach**:
  1. Add the missing 9 methods to the `ArawnService` trait.
  2. Define response structs: `MemoryStoreResult`, `MemorySummary`, `CommandList`, `InventoryResult`, `PromotionResult`.
  3. Map `ServiceError` variants to distinct RPC error codes: `"not_found"`, `"invalid_operation"`, `"engine_error"`, `"storage_error"`, `"internal_error"`.
  4. Update the WebSocket handler to call through the trait for all methods.
- **Dependencies**: None, but pairs well with R-17.

---

### R-17: Refactor Monolithic Orchestration Functions

- **Addresses**: LEG-003, LEG-004, EVO-09, XC-05, OPS-03
- **Severity of addressed findings**: Major (LEG-003, LEG-004)
- **Effort**: Days
- **What to do**: Extract serve mode initialization into dedicated functions. Break `send_message` into composable helpers. Add `#[instrument]` span annotations.
- **Why it matters**: These two functions are the primary wiring for the entire system. Their length makes modification, debugging, and tracing difficult.
- **Suggested approach**:
  1. Extract `fn register_default_tools(registry, config, bg_manager)` from `main.rs`.
  2. Extract `fn build_serve_context(config, store) -> ServeContext` that groups the serve-mode components.
  3. In `local_service.rs`, extract `fn load_session_state(...)`, `fn build_session_context(...)`, `fn build_engine(...)`.
  4. Add `#[instrument(skip_all, fields(session_id))]` to `send_message` and key engine methods for automatic span context.
- **Dependencies**: None, but easier after R-15 (smaller crate boundaries make extraction clearer).

---

### R-18: Deprecate Legacy Plugin System

- **Addresses**: LEG-005, EVO-08
- **Severity of addressed findings**: Low
- **Effort**: Hours (deprecation notice), Weeks (full removal)
- **What to do**: Add deprecation notices to the legacy WASM plugin system. Feature-gate the `fidius` dependency. Establish a migration timeline.
- **Why it matters**: Two complete plugin systems create maintenance cost and contributor confusion. The naming similarity (`plugin_loader.rs` vs `plugins/loader.rs`) is actively misleading.
- **Suggested approach**:
  1. Add `#[deprecated]` and prominent `//! DEPRECATED` module docs to `plugin_loader.rs`, `plugin_adapter.rs`, `plugin_watcher.rs`.
  2. Add a `legacy-plugins` feature flag in `arawn-engine/Cargo.toml`. Gate the fidius dependency and legacy plugin loading behind it.
  3. Update `main.rs` to log a deprecation warning when legacy plugins are loaded.
  4. Set a removal target date.
- **Dependencies**: None.

---

### R-19: Add RPC Protocol Versioning

- **Addresses**: API-004, API-005, API-001
- **Severity of addressed findings**: Medium
- **Effort**: Days
- **What to do**: Add a `hello`/`handshake` method returning the server version and supported methods. Standardize RPC method naming.
- **Why it matters**: The server and TUI are separate binaries that may be updated independently. Without versioning, any RPC surface change risks silent client breakage.
- **Suggested approach**:
  1. Add a `hello` RPC method returning `{ version: "0.x.y", methods: [...] }`.
  2. Standardize all method names to `verb_noun` pattern: rename `remember_fact` to `store_memory`, `forget_entity` to `delete_memory`, `list_available_commands` to `list_commands`, `memory_summary` to `get_memory_summary`.
  3. Wrap streamed events in the Response envelope for protocol consistency, or document the state machine explicitly.
  4. Clients should treat unknown event tags as no-ops.
- **Dependencies**: R-16 (service trait completion) should happen first so the method list is stable.

---

### R-20: Add CLI Argument Parsing via Clap

- **Addresses**: API-009
- **Severity of addressed findings**: Low
- **Effort**: Hours
- **What to do**: Replace the hand-rolled argument parsing in `main.rs` with `clap`. This provides `--help`, `--version`, shell completions, and validation.
- **Why it matters**: Missing `--help`, silent defaults on malformed `--port`, and no shell completion limit discoverability.
- **Suggested approach**: Define a `#[derive(Parser)]` struct with subcommands for `serve`, `tui`, `plugin`. The current argument surface is small enough for a straightforward migration.
- **Dependencies**: None.

## Architectural Recommendations

Systemic improvements beyond individual findings.

---

### R-21: Establish a "Fail Visible" Convention

- **Addresses**: SP-1 (Silent Degradation pattern), COR-003, SEC-006, OPS-02, OPS-07, OPS-10
- **Severity of addressed findings**: Various (High to Low)
- **Effort**: Days (convention + initial sweep)
- **What to do**: Establish a project convention that failures affecting data integrity or security must be surfaced to the user. Add an `EngineEvent::Warning` variant for non-fatal but user-visible problems. Audit all `let _ =`, `if let Err(e) { log }`, and `unwrap_or(default)` sites.
- **Why it matters**: The silent degradation pattern is the most pervasive systemic issue. Seven independent findings describe the same behavior: the system continues as if nothing happened when things fail, creating a gap between apparent and actual system state.
- **Suggested approach**:
  1. Add `EngineEvent::Warning { message: String }` to the streaming protocol.
  2. Categorize existing silent-failure sites into: (a) must surface to user (persistence errors, sandbox failures), (b) should log at warn (parse fallbacks, stats update failures), (c) acceptable to silence (truly inconsequential).
  3. Document the convention in a CONTRIBUTING.md or code comment: "Data integrity and security failures MUST emit a Warning or Error event."
- **Dependencies**: None.

---

### R-22: Replace String-Typed Dispatch with Enum/Trait-Based Dispatch

- **Addresses**: SP-2 (String-typed dispatch pattern), LEG-009, LEG-010, EVO-06
- **Severity of addressed findings**: High (LEG-009 adjusted)
- **Effort**: Weeks
- **What to do**: Replace string-based tool name matching in filter constants and permission categories with enum-based or trait-based dispatch that is verified at compile time.
- **Why it matters**: String matching for dispatch is the root cause of two silent correctness bugs (LEG-009, LEG-010) and prevents compile-time verification of name consistency across the system.
- **Suggested approach**:
  1. Add a `category()` method to the `Tool` trait returning a `ToolCategory` enum.
  2. Replace the `CORE_TOOLS`, `TASK_TOOLS`, etc. filter constants with trait-based queries: `registry.tools_by_category(ToolCategory::Task)`.
  3. For RPC dispatch, consider a macro-generated dispatch table or a `HashMap<&str, Box<dyn Handler>>` pattern.
  4. For permission checking, use `tool.category()` instead of `tool_category(name)`.
- **Dependencies**: R-15 (extracting `arawn-tool` crate provides the natural home for `ToolCategory`).

## Summary Roadmap

### Phase 1: Bug Fixes and Security Hardening (Days)
1. **R-01**: Fix tool name casing mismatches (Hours)
2. **R-02**: Add path restrictions to grep/glob (Hours)
3. **R-05**: Fix session grants to respect deny rules (Hours)
4. **R-04**: Add per-session lock (Hours)
5. **R-06**: Surface persistence errors (Hours)
6. **R-14**: Fix `truncate_input` UTF-8 panic (Hours)
7. **R-12**: Log malformed LLM arguments (Hours)

### Phase 2: Operational Reliability (Days)
8. **R-03**: Implement sandbox for background shell (Days)
9. **R-07**: Implement cancellation (Days)
10. **R-08**: Harden JSONL persistence (Days)
11. **R-09**: Add WebSocket authentication (Days)
12. **R-11**: Fix promotion atomicity (Hours)
13. **R-13**: Add signal handling and graceful shutdown (Days) -- depends on R-07

### Phase 3: Structural Improvements (Weeks)
14. **R-10**: Surface sandbox failures and hook security (Days)
15. **R-15**: Extract `arawn-tool` interface crate (Weeks)
16. **R-16**: Complete the `ArawnService` trait (Days)
17. **R-17**: Refactor monolithic orchestration functions (Days)
18. **R-18**: Deprecate legacy plugin system (Hours + ongoing)

### Phase 4: API Polish (Days-Weeks)
19. **R-19**: Add RPC protocol versioning (Days) -- depends on R-16
20. **R-20**: Add CLI argument parsing via clap (Hours)
21. **R-21**: Establish "fail visible" convention (Days)
22. **R-22**: Replace string-typed dispatch (Weeks) -- depends on R-15

Items within each phase can generally be parallelized. Phase dependencies are noted: R-13 depends on R-07 for cancellation tokens; R-19 depends on R-16 for a stable method list; R-22 depends on R-15 for the `arawn-tool` crate.
