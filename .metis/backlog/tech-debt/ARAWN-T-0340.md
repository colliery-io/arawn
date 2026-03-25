---
id: tech-debt-audit-comprehensive-code
level: task
title: "Tech Debt Audit: Comprehensive Code Quality Report (March 2026)"
short_code: "ARAWN-T-0340"
created_at: 2026-03-22T00:34:45.586498+00:00
updated_at: 2026-03-25T13:39:25.025932+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Tech Debt Audit: Comprehensive Code Quality Report (March 2026)

Full codebase audit of Arawn (333 Rust files, ~135K lines, 19 workspace crates).

---

## Tech Debt Inventory (Ranked by Severity)

### Critical (Blocks Reliability/Security)

#### C1. `unwrap()` on `current_turn_mut()` in Agent Turn Loop

- **File**: `/Users/dstorey/Desktop/arawn/crates/arawn-agent/src/agent.rs`, lines 215, 291, 392
- **What**: The core agent turn loop calls `session.current_turn_mut().unwrap()` in three places. If the session ever has no current turn (e.g., due to a race condition or unexpected state), the server process panics.
- **Why it matters**: This is the hottest code path in the entire system -- every LLM interaction flows through it. A panic here kills the server for all users.
- **Fix**: Return `AgentError::Internal("no current turn")` instead of unwrapping. The session should always have a current turn at these points, but defensive error propagation is critical for a server process.
- **Effort**: S

#### C2. `expect()` in Production HTTP Client Construction

- **File**: `/Users/dstorey/Desktop/arawn/crates/arawn-agent/src/tools/web.rs`, lines 65, 76, 718, 728
- **What**: `WebFetchTool::new()` and `WebFetchTool::with_config()` both call `.expect("Failed to build HTTP client")` on `reqwest::Client::builder().build()`.
- **Why it matters**: If client construction fails (e.g., TLS backend unavailable, invalid config), the entire server panics rather than gracefully degrading.
- **Fix**: Return `Result` from constructors, or make `WebFetchTool` creation fallible.
- **Effort**: S

#### C3. `expect()` in Command Validator Regex Compilation

- **File**: `/Users/dstorey/Desktop/arawn/crates/arawn-agent/src/tool/command_validator.rs`, line 75
- **What**: `regex::Regex::new(pat).expect("invalid blocked pattern regex")` -- panics if any blocked pattern regex is invalid.
- **Why it matters**: These regexes are compile-time constants so this is low-risk today, but if patterns become configurable (from plugins or config), a bad regex would crash the server.
- **Fix**: Use `lazy_static` or `std::sync::OnceLock` with pre-validated regexes, or propagate the error.
- **Effort**: S

#### C4. Daemon Mode Advertised But Not Implemented

- **File**: `/Users/dstorey/Desktop/arawn/crates/arawn/src/commands/start.rs`, lines 105-109
- **What**: `arawn start -d` prints "Daemon mode not yet implemented" and returns. The `-d` flag is publicly advertised in help text.
- **Why it matters**: Users attempting production deployment with daemonization get a silent no-op. This is a feature gap that should either be implemented or the flag removed.
- **Fix**: Either implement daemonization (using `daemonize` crate or systemd integration) or remove the flag and document the recommended approach (systemd unit file).
- **Effort**: M

### High (Significant Maintenance Burden)

#### H1. Monolithic `run()` Function in `start.rs` (1,405 Lines)

- **File**: `/Users/dstorey/Desktop/arawn/crates/arawn/src/commands/start.rs`, lines 104-1509
- **What**: The `run()` function that starts the server is a single 1,405-line function. It handles config loading, LLM backend creation, pipeline engine setup, memory store initialization, tool registration, plugin system, MCP servers, agent construction, session indexer, workstream manager, and server startup.
- **Why it matters**: This function is extremely difficult to test, reason about, or modify without risk of breaking unrelated subsystems. It has 314 deeply-nested lines (7+ levels of indentation). The function uses 73 `.clone()` calls.
- **Fix**: Extract into a `ServerBuilder` with methods like `init_pipeline()`, `init_plugins()`, `init_mcp()`, `init_memory()`, `build_agent()`, etc. Each subsystem becomes independently testable.
- **Effort**: L

#### H2. Monolithic TUI `App` Struct (3,272 Lines, 95 `pub` Items)

- **File**: `/Users/dstorey/Desktop/arawn/crates/arawn-tui/src/app.rs`
- **What**: The `App` struct contains all TUI state and behavior in a single file. `handle_server_message` is 235 lines, `handle_input_key` is 204 lines. The struct exposes 95 public items.
- **Why it matters**: Adding any TUI feature requires modifying this single file, increasing merge conflict risk and cognitive load.
- **Fix**: Extract into separate modules: `App` (core state), `MessageHandler` (server message processing), `KeyHandler` (input dispatch), `SidebarState`, `SessionPanel`. Use a trait-based component system.
- **Effort**: L

#### H3. Duplicated LLM Backend Boilerplate

- **Files**: `/Users/dstorey/Desktop/arawn/crates/arawn-llm/src/openai.rs` (1,594 lines), `/Users/dstorey/Desktop/arawn/crates/arawn-llm/src/anthropic.rs` (1,199 lines)
- **What**: Both backends duplicate: config structs with identical fields (timeout, retries, backoff), retry logic, SSE stream parsing infrastructure, HTTP client construction, and error mapping. Constants `DEFAULT_TIMEOUT_SECS`, `DEFAULT_MAX_RETRIES`, `DEFAULT_RETRY_BACKOFF_MS` are duplicated verbatim.
- **Why it matters**: Any change to retry logic, timeout handling, or streaming must be applied in both places. Adding a new backend (e.g., Google Gemini) requires duplicating ~800 lines.
- **Fix**: Extract `BackendConfig` struct, shared SSE parser, and `HttpLlmBackend<T: ApiFormat>` generic wrapper. The `with_retry` function already exists in `backend.rs` but config/construction is duplicated.
- **Effort**: M

#### H4. Inconsistent `dirs` Crate Versions Across Workspace

- **Files**: Multiple `Cargo.toml` files
- **What**: Three different versions of the `dirs` crate are used:
  - `dirs = "5.0"` in `arawn-agent`, `arawn-llm`
  - `dirs = "6.0"` in `arawn-config`, `arawn-plugin`, `arawn` (CLI)
  - `dirs = { workspace = true }` (resolves to `"6.0"`) in `arawn-sandbox`, `arawn-server`, `arawn-workstream`
- **Why it matters**: Version 5 and 6 may return different paths on some platforms. Using different versions in the same process can cause config/data directory mismatches. Also increases binary size with duplicate crate trees.
- **Fix**: Standardize all crates to `dirs = { workspace = true }` (version 6.0).
- **Effort**: S

#### H5. `ort` Dependency on Release Candidate

- **File**: `/Users/dstorey/Desktop/arawn/Cargo.toml`, line 78
- **What**: `ort = { version = "2.0.0-rc.11", ... }` -- the ONNX Runtime binding is pinned to a release candidate.
- **Why it matters**: RC versions may have breaking changes before stable release. The API may shift, requiring rework. Semantic versioning guarantees do not apply to pre-release versions.
- **Fix**: Monitor for stable 2.0.0 release and upgrade. If blocking, consider pinning to exact version with `=2.0.0-rc.11` to prevent accidental upgrades.
- **Effort**: S (when stable is released)

#### H6. 28 Error Types Across the Workspace

- **Files**: Every crate has its own error type (28 total error enums found)
- **What**: Some crates have multiple error types (e.g., `arawn-config` has `ConfigError`, `SecretStoreError`, `AgeError`; `arawn-llm` has `LlmError`, `ResponseValidationError`). The `DomainError` wraps some but uses `String` for others (`Mcp(String)`, `Config(String)`, `Internal(String)`).
- **Why it matters**: String-based error variants lose type information and make programmatic error handling impossible. 16 `Result<T>` type aliases can shadow each other when multiple crate errors are in scope.
- **Fix**: Use `#[from]` consistently in `DomainError` instead of `String` wrappers. Consolidate per-crate error types where they overlap (e.g., `SecretStoreError` could be a variant of `ConfigError`).
- **Effort**: M

### Medium (Code Quality Concerns)

#### M1. No Periodic Cleanup for `WsConnectionTracker`

- **File**: `/Users/dstorey/Desktop/arawn/crates/arawn-server/src/state.rs`, lines 200-269
- **What**: `WsConnectionTracker` has a `cleanup()` method but it is never called on a schedule. Cleanup only happens opportunistically during `check_rate()`. If an IP connects once and never again, its entry stays in the `HashMap` forever.
- **Why it matters**: Memory leak proportional to unique IPs over server lifetime. In a long-running server, this map grows unboundedly.
- **Fix**: Spawn a periodic cleanup task (e.g., every 5 minutes) in the server startup, similar to how `cleanup_expired_pending_reconnects` is called in WS handlers.
- **Effort**: S

#### M2. Hardcoded Default Server URL

- **File**: `/Users/dstorey/Desktop/arawn/crates/arawn/src/main.rs`, line 140
- **What**: `"http://localhost:8080".to_string()` is hardcoded as the fallback server URL for CLI commands.
- **Why it matters**: If the server default port changes (via `DEFAULT_PORT` constant), the CLI default will not match. The port 8080 should reference the same constant used by the server.
- **Fix**: Use `format!("http://localhost:{}", arawn_types::config::defaults::DEFAULT_PORT)`.
- **Effort**: S

#### M3. Hardcoded Fallback Timeout in Pipeline ScriptExecutor

- **File**: `/Users/dstorey/Desktop/arawn/crates/arawn/src/commands/start.rs`, line 462
- **What**: `ScriptExecutor::new(fallback_cache, std::time::Duration::from_secs(30))` uses a magic number 30 for the fallback timeout.
- **Why it matters**: This fallback path bypasses the configured `pipeline_cfg.task_timeout_secs`, so the fallback executor uses a different timeout than the primary.
- **Fix**: Use `pipeline_cfg.task_timeout_secs` for both primary and fallback executors.
- **Effort**: S

#### M4. Missing WebSocket Alert on Workstream Rename

- **File**: `/Users/dstorey/Desktop/arawn/crates/arawn-server/src/routes/workstreams.rs`, line 764
- **What**: `// TODO: Send WebSocket alert if renamed` -- connected TUI clients will not see workstream renames until they refresh.
- **Why it matters**: Creates stale UI state for real-time clients.
- **Fix**: Broadcast a `WorkstreamRenamed` event over the WebSocket channel after successful rename.
- **Effort**: S

#### M5. `unimplemented!()` in Vendored Code

- **Files**: `/Users/dstorey/Desktop/arawn/crates/orp-vendored/src/pipeline.rs` line 74, `/Users/dstorey/Desktop/arawn/crates/orp-vendored/src/model.rs` line 147
- **What**: `Composable::apply()` implementations panic with `unimplemented!()`.
- **Why it matters**: If any code path calls `apply()` on these types, the server panics. Since these are vendored, they are under Arawn's control.
- **Fix**: Either return a proper error, or add `#[doc(hidden)]` and ensure the trait is never dispatched dynamically.
- **Effort**: S

#### M6. 16 `#[allow(dead_code)]` Annotations in Production Code

- **File**: `/Users/dstorey/Desktop/arawn/crates/arawn/src/client/mod.rs` (10 annotations), plus others in `arawn-pipeline`, `arawn-tui`, `arawn-llm`
- **What**: Many of these are marked "Public API not yet called" or "not yet wired to CLI command", indicating features that were partially built.
- **Why it matters**: Dead code increases compile time, binary size, and cognitive load. Some may represent incomplete feature work that should be tracked.
- **Fix**: Audit each annotation. Remove truly dead code, or create backlog tickets for incomplete features. The client module has 10 such annotations, suggesting the CLI client layer is partially built.
- **Effort**: M

#### M7. Large Server Route Files

- **Files**: `workstreams.rs` (1,660 lines), `sessions.rs` (1,557 lines), `memory.rs` (1,107 lines), `mcp.rs` (989 lines)
- **What**: Route handler files in `arawn-server` contain request types, response types, handler functions, validation logic, and conversion helpers all in one file.
- **Why it matters**: Difficult to navigate and increases merge conflict risk.
- **Fix**: Split into `types.rs`, `handlers.rs`, and `conversions.rs` per route module, or extract shared validation/pagination into utility modules.
- **Effort**: M

#### M8. Inline Tests in Large Production Files

- **Files**: Multiple files have large test modules embedded:
  - `app.rs`: 1,046 test lines (32% of file)
  - `types.rs` (config): 1,399 test lines (45% of file)
  - `state.rs`: 1,034 test lines (45% of file)
  - `agent.rs`: 983 test lines (47% of file)
  - `subscription.rs`: 1,024 test lines (57% of file)
- **What**: Test code bloats these already-large files.
- **Why it matters**: Makes the production code harder to navigate; IDE performance suffers.
- **Fix**: Move test modules to separate files using `#[path = "..."]` or the `tests/` convention.
- **Effort**: M

### Low (Cleanup Tasks)

#### L1. RLM Config Mapping is Manual and Fragile

- **File**: `/Users/dstorey/Desktop/arawn/crates/arawn/src/commands/start.rs`, lines 997-1022
- **What**: Each field of `RlmTomlConfig` is manually mapped to `RlmConfig` with individual `if let Some(...)` blocks. If a field is added to the TOML config but not mapped here, it silently does nothing.
- **Fix**: Implement `From<RlmTomlConfig> for RlmConfig` or use a merge pattern.
- **Effort**: S

#### L2. Duplicate Tool Name Registration

- **File**: `/Users/dstorey/Desktop/arawn/crates/arawn/src/commands/start.rs`, lines 395-412
- **What**: Tool output config is set for both aliases (e.g., `"shell"` and `"bash"`, `"file_read"` and `"read_file"`), suggesting tool naming inconsistency.
- **Fix**: Standardize on canonical tool names and resolve aliases at the registry level.
- **Effort**: S

#### L3. Plugin CLI Tools and Prompt Fragments Not Implemented

- **File**: `/Users/dstorey/Desktop/arawn/crates/arawn/src/commands/start.rs`, line 789
- **What**: `// Note: CLI tools (commands/) and prompt fragments are not yet implemented`
- **Fix**: Either implement or remove the placeholder comment and track as a backlog item.
- **Effort**: M (to implement)

#### L4. Workflow Engine Missing Unregister Method

- **File**: `/Users/dstorey/Desktop/arawn/crates/arawn/src/commands/start.rs`, lines 614-618
- **What**: `// Engine doesn't have an unregister method yet` -- when a workflow file is deleted during hot-reload, the workflow remains registered.
- **Fix**: Add `PipelineEngine::unregister_workflow()`.
- **Effort**: S

---

## Code Quality Issues

### Unwrap/Expect in Production Code Paths

| Location | Call | Risk |
|---|---|---|
| `agent.rs:215,291,392` | `current_turn_mut().unwrap()` | Server crash on missing turn |
| `web.rs:65,76,718,728` | `Client::builder().build().expect(...)` | Crash on TLS/config failure |
| `command_validator.rs:75` | `Regex::new(pat).expect(...)` | Crash if patterns become configurable |
| `compaction.rs:547` | `.expect("should have system prompt")` | Crash if prompt builder misconfigured |
| `stream.rs:546` | `chunks.last().expect(...)` | Crash on empty stream |

All other `unwrap()`/`expect()` calls found are in test code (`#[cfg(test)]`, `tests/` directories, or `arawn-test-utils`), which is appropriate.

### Functions Needing Decomposition

| File | Function | Lines | Issue |
|---|---|---|---|
| `start.rs` | `run()` | 1,405 | Entire server initialization in one function |
| `app.rs` | `handle_server_message()` | 235 | All message types in one match |
| `app.rs` | `handle_input_key()` | 204 | Complex key dispatch logic |
| `agent.rs` | `turn()` | ~200 | Core turn loop with deeply nested error handling |

### Duplicated Logic

1. **SSE Stream Parsing**: Both `openai.rs` and `anthropic.rs` implement SSE parsing (`parse_openai_sse_stream` vs `parse_sse_stream`). Extract a shared `SseParser`.
2. **HTTP Client Construction**: `reqwest::Client::new()` is called independently in 8+ locations across `arawn-oauth` (3 places), `arawn-agent/tools/web.rs` (4 places), and `arawn/src/client/mod.rs`. Consider a shared HTTP client factory.
3. **Config Path Resolution**: The pattern `if p.is_relative() { data_dir.join(p) } else { p }` appears 5+ times in `start.rs`. Extract a `resolve_config_path()` helper.

### Inconsistent Patterns

1. **Error Handling**: Some crates use `thiserror` with typed errors, while `DomainError` uses `String` for some variants (`Mcp(String)`, `Config(String)`), losing type information.
2. **`anyhow` vs typed errors**: 87 uses of `anyhow::` in production code, mostly in the CLI crate. The server and domain layers correctly use typed errors, but the boundary is fuzzy.

---

## Architecture Concerns

### Crate Boundaries

1. **`arawn-domain` is underutilized**: The domain crate (1,370 lines) is mostly a re-export facade (69 lines of `pub use` in `lib.rs`). The actual domain services (`DomainServices`) is used by the server but most business logic still lives in `arawn-server/src/state.rs` (2,298 lines) and the route handlers. Consider moving more orchestration logic from server routes into domain services.

2. **`arawn-types` vs `arawn-config`**: `arawn-types` (1,128 lines) contains config defaults, hooks, delegation types, and `FsGate` -- these seem split arbitrarily from `arawn-config`. The two crates have overlapping concerns.

3. **The CLI crate (`arawn`) does too much**: The `start.rs` file (2,017 lines) contains server initialization logic that should live in `arawn-server` or `arawn-domain`. The CLI should only parse args and delegate.

### Circular Dependency Risks

No circular dependencies exist currently, but the heavy re-export pattern in `arawn-domain` (importing from 7 crates and re-exporting) creates a risk: any new dependency between the re-exported crates could create a cycle. The domain crate's lib.rs should be reviewed when adding cross-crate dependencies.

### Abstraction Issues

1. **Too thin**: `arawn-domain` adds minimal value as a re-export layer. It should either gain more business logic or be dissolved.
2. **Too thick**: `arawn-server/src/state.rs` (2,298 lines) contains too much: session ownership, reconnection, task tracking, rate limiting, and domain orchestration all in one file.

---

## Dependency Audit

### Pre-Release Dependencies

| Dependency | Version | Issue |
|---|---|---|
| `ort` | `2.0.0-rc.11` | Release candidate; no semver guarantees |

### Version Inconsistencies

| Dependency | Versions Used | Crates Affected |
|---|---|---|
| `dirs` | `5.0`, `5`, `6.0`, workspace (`6.0`) | arawn-agent, arawn-llm, arawn-config, arawn-plugin |

### Dependencies Worth Reviewing

1. **`portable-pty = "0.8"`**: Used for shell tool PTY support. This crate has low maintenance activity. Consider if `tokio::process::Command` with pipe-based I/O would suffice.
2. **`scraper = "0.20"`**: HTML parsing for web tool. Adequate but `select.rs` or `html5ever` directly may have better maintained APIs.
3. **`ort` with `download-binaries`**: Downloads ~200MB of ONNX Runtime binaries at build time. This dramatically increases build times and CI costs. The `local-embeddings` feature should be more prominently documented as optional.

### Vendored Crates

Two vendored crates (`orp-vendored`, `gline-rs-vendored`) are excluded from the workspace. These contain `unimplemented!()` stubs that could panic. Since they're vendored, they should be treated as owned code and fixed.

---

## Summary Statistics

| Metric | Count |
|---|---|
| Total lines of Rust | ~135,287 |
| Workspace crates | 19 (+ 2 vendored) |
| Files over 1,000 lines | 22 |
| Files over 2,000 lines | 4 |
| Error types | 28 |
| Result type aliases | 16 |
| `#[allow(dead_code)]` annotations | 16 |
| Production `unwrap()`/`expect()` calls | ~12 |
| TODO/FIXME comments | 2 |
| `unimplemented!()` calls | 2 |

## Status Updates

### 2026-03-25: Triage + Quick Fixes

Most items from the original audit are already resolved by initiatives I-0029 through I-0038:
- **C1** (unwrap in agent turn): Fixed in I-0030
- **C4** (daemon mode): Implemented in I-0032
- **H1** (start.rs 1,405 lines): Decomposed to 245 lines in I-0035
- **H2** (TUI App 3,272 lines): Decomposed to 452 lines in I-0036
- **H3** (LLM backend duplication): Deduplicated in I-0037
- **H4** (dirs crate versions): Consolidated in I-0034
- **H5** (ort RC): Noted, awaiting stable release
- **H6** (28 error types): Partially cleaned in I-0034
- **M1** (WsConnectionTracker leak): Fixed in I-0034
- **M2** (hardcoded server URL): Fixed in I-0034

Fixed today:
- **C2**: Removed `expect()` from WebFetchTool/WebSearchTool Default impls → graceful fallback
- **C3**: Removed `expect()` from CommandValidator regex → log + skip
- **M3**: Fixed hardcoded 30s fallback timeout → uses `pipeline_cfg.task_timeout_secs`
- **M4**: Removed stale TODO, added tracing for file promotion renames
- **graphqlite**: Upgraded 0.3.2 → 0.3.10, fixing 2 broken graph store tests

**Also fixed:**
- M5: Replaced `unimplemented!()` with `Err()` in vendored orp Composable impls
- M6: Audited 16 annotations → removed 4 incorrect, 12 remaining are legitimate (serde + JoinHandle)
- graphqlite upgraded 0.3.2 → 0.3.10 fixing 2 graph store test failures

**Deferred (structural refactors, own initiative if pursued):**
- M7: Large server route files (1.1K-1.7K lines) — future decomposition candidate
- M8: Inline tests in large production files — cosmetic

## Recommended Priority Order

1. **C1-C3**: Fix production panics (unwrap/expect in hot paths) -- 1 day
2. **H4**: Standardize `dirs` crate versions -- 30 minutes
3. **M1-M3**: Fix memory leak and hardcoded values -- 2 hours
4. **H1**: Extract `ServerBuilder` from `start.rs` -- 2-3 days
5. **H2**: Decompose TUI `App` -- 2-3 days
6. **H3**: Unify LLM backend boilerplate -- 1-2 days
7. **H6**: Clean up error type hierarchy -- 1-2 days
8. **M6-M8**: Dead code cleanup and file splitting -- 2-3 days