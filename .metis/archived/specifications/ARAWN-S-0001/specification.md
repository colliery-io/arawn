---
id: comprehensive-testing-plan-and
level: specification
title: "Comprehensive Testing Plan and Coverage Gap Analysis"
short_code: "ARAWN-S-0001"
created_at: 2026-03-22T00:33:39.913278+00:00
updated_at: 2026-03-22T00:33:39.913278+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Comprehensive Testing Plan and Coverage Gap Analysis

## Overview

This specification provides an exhaustive analysis of test coverage across the entire Arawn workspace (20 crates, 333 source files, ~107,000 lines of Rust) and defines a prioritized plan to close all critical coverage gaps. The analysis is based on 3,196 existing tests (plus 109 ignored/doc tests), existing LCOV coverage data, and a manual audit of every crate.

---

## 1. Current Test Coverage Assessment

### 1.1 Summary Statistics

| Metric | Value |
|--------|-------|
| Total crates | 20 (+ 6 runtimes, 2 vendored) |
| Total source files | 333 |
| Total tests (cargo test --list) | 3,196 |
| Ignored/doc tests | 109 |
| Integration test files | 28 |
| Combined line coverage (measured crates) | 68.7% |
| Crates with NO coverage measurement | 6 (arawn-agent, arawn-server, arawn-llm, arawn-oauth, arawn-plugin, arawn-workstream) |

### 1.2 Per-Crate Coverage Breakdown

#### Crates WITH LCOV coverage data

| Crate | LOC | Tests | Line Coverage | Assessment |
|-------|-----|-------|---------------|------------|
| arawn-client | 2,091 | 4 unit + 68 integration | 93.4% (674/721) | Good. Client API surface well covered via integration tests. |
| arawn-config | 6,249 | 171 | 91.7% (3030/3302) | Good. Config parsing, secrets, discovery all tested. |
| arawn-domain | 1,370 | 67 | 93.6% (762/814) | Good. Service layer well tested. |
| arawn-mcp | 2,202 | 44 unit + 19 integration | 87.3% (887/1015) | Adequate. Transport and protocol well tested; client less so. |
| arawn-memory | 7,854 | 172 | 93.0% (4176/4487) | Good. Store ops, vector, graph, validation all covered. |
| arawn-pipeline | 5,617 | 177 unit + 18+3 integration | 79.7% (3156/3955) | Moderate. Engine and definition solid; sandbox/loader weaker. |
| arawn-sandbox | 1,318 | 31 | 85.4% (635/743) | Adequate. Manager and platform tested; many tests self-skip on CI (sandbox unavailable). |
| arawn-script-sdk | 462 | 18 | 75.6% (168/222) | Moderate. Basic SDK tested; some edge cases missing. |
| arawn-session | 1,135 | 18 | 75.8% (452/596) | Moderate. Cache well tested; persistence.rs has ZERO inline tests. |
| arawn-tui | 9,811 | 153 | 52.5% (3199/6088) | WEAK. UI rendering modules almost entirely untested. |
| arawn-types | 1,128 | 14 | 86.7% (229/264) | Adequate. Config/hooks types tested; delegation/fs_gate have no inline tests. |
| arawn (CLI) | 7,801 | 60 unit + 25+43 integration | 23.9% (1113/4655) | CRITICAL. Most command implementations untested at unit level. |

#### Crates WITHOUT coverage measurement (largest gap)

| Crate | LOC | Tests | Assessment |
|-------|-----|-------|------------|
| **arawn-agent** | **22,381** | 562 | Largest crate. Has extensive unit tests for agent loop, compaction, context, error, indexing, NER. Tool gate/execution/validation tested. BUT: shell tool, web tool, file tool, explore tool, delegate tool, note tool, workflow tool, catalog tool have no unit tests beyond what integration tests catch. |
| **arawn-server** | **15,048** | 294 unit + 175 integration/e2e | Heavy integration test coverage. Auth, ratelimit, pagination, state, session_cache all have unit tests. BUT: ws/connection.rs has NO tests. routes/logs.rs, routes/openapi.rs, config.rs have NO tests. |
| **arawn-llm** | **8,086** | 209 | Anthropic/OpenAI backends, embeddings, client, types all tested. Good mock support via MockBackend. |
| **arawn-oauth** | **2,029** | 49 | Token manager, proxy, OAuth flow tested. Passthrough has tests. Adequate. |
| **arawn-plugin** | **9,071** | 279 | Manager, manifest, skill, subscription, validation, watcher, hooks, agent_spawner all tested. Good. |
| **arawn-workstream** | **9,382** | 326 | Directory ops, fs_gate, message_store, path_validator (with proptest!), session, manager all well tested. Good. |

### 1.3 Test Type Distribution

| Type | Count | Files | Notes |
|------|-------|-------|-------|
| Unit tests (#[test] in src/) | ~2,800 | 181 files with #[cfg(test)] | Core logic coverage |
| Integration tests (tests/) | ~350 | 28 files | Server, client, MCP, pipeline, CLI |
| End-to-end tests | ~150 | 7 files (e2e_*) | Server scenarios, WebSocket, stress |
| Doc tests | ~109 | Inline | All marked #[ignore] to avoid requiring live backends |
| Property-based tests | 5 | 1 file (path_validator) | Only arawn-workstream uses proptest |

### 1.4 Test Infrastructure

- **arawn-test-utils** crate (2,087 LOC): Provides `TestServer`, `MockBackend`, `ScriptedMockBackend`, `MockToolSet`, SSE helpers, WebSocket client, assertion helpers.
- **CI pipeline** (`.github/workflows/ci.yml`): Runs `cargo test --workspace -- --test-threads=1` plus ignored tests. No coverage measurement in CI.
- **Coverage tooling** (`coverage/` directory): LCOV files exist for 12 crates but are manually generated and NOT part of CI. Six of the largest crates (agent, server, llm, oauth, plugin, workstream) have no coverage data at all.

---

## 2. Critical Coverage Gaps (Ranked by Risk)

### RISK LEVEL: CRITICAL

#### 2.1 WebSocket Connection Lifecycle (`arawn-server/src/routes/ws/connection.rs`) - NO UNIT TESTS

- **File**: `crates/arawn-server/src/routes/ws/connection.rs`
- **Functions untested**: `handle_socket()` (line 86), `send_message()` (line 226), `ConnectionState::new()` (line 62)
- **Risk**: WebSocket is the primary real-time communication channel. The `handle_socket` function manages authentication, idle timeouts, message routing, and cancellation tokens. Bugs here cause silent connection drops, auth bypass, or resource leaks.
- **Impact**: Data loss, denial of service, potential auth bypass during WebSocket upgrade.

#### 2.2 Shell Command Execution (`arawn-agent/src/tools/shell.rs`) - Tests exist but shallow

- **File**: `crates/arawn-agent/src/tools/shell.rs` (~600 LOC)
- **What's tested**: `ShellConfig` defaults, blocked command list, PTY size.
- **What's NOT tested**:
  - Actual command execution through `ShellTool::call()` with sandbox integration
  - Blocked command bypass attempts (e.g., encoding tricks, shell metacharacter injection)
  - PTY mode execution path
  - Output truncation at `max_output_size`
  - Timeout behavior during execution
  - Working directory validation
- **Risk**: This is the most security-sensitive tool - it executes arbitrary shell commands. Insufficient testing of the blocklist bypass and sandbox enforcement could allow destructive operations.

#### 2.3 CLI Command Implementations (`arawn/src/commands/*.rs`) - 23.9% coverage

- **Files**: 17 command modules in `crates/arawn/src/commands/`
- **Tested**: `mod.rs` error formatting (17 tests), basic CLI arg parsing via integration tests
- **NOT tested at unit level**: `ask.rs`, `chat.rs`, `agent.rs`, `mcp.rs`, `memory.rs`, `notes.rs`, `plugin.rs`, `repl.rs`, `secrets.rs`, `session.rs`, `start.rs`, `status.rs`, `tui.rs`, `auth.rs`, `config.rs`, `logs.rs`, `output.rs`
- **Risk**: These are the user-facing entry points. Error handling, output formatting, and server communication logic are untested. The `start` command starts the server - bugs here affect every user.

#### 2.4 Session Persistence (`arawn-session/src/persistence.rs`) - ZERO TESTS

- **File**: `crates/arawn-session/src/persistence.rs` (114 lines)
- **What's untested**: `PersistenceHook` trait default implementations, `NoPersistence` struct, `SessionData` constructors and builder methods.
- **Risk**: Session persistence is the mechanism for surviving server restarts. While the trait is simple, the `on_evict` default and `SessionData` serialization path are untested.

#### 2.5 OAuth Passthrough Forwarding (`arawn-oauth/src/passthrough.rs`) - Limited tests

- **File**: `crates/arawn-oauth/src/passthrough.rs`
- **Functions with tests**: `extract_api_key()`, basic config
- **Functions WITHOUT tests**: `forward_raw()` (line 127), `forward_raw_stream()` (line 174) - these make actual HTTP requests upstream
- **Risk**: The passthrough forwards authenticated requests to Anthropic's API. Bugs in header forwarding, token injection, or stream handling could leak credentials or corrupt responses.

### RISK LEVEL: HIGH

#### 2.6 Tool Filesystem Gate (`arawn-agent/src/tool/gate.rs`) - Tests compile but don't run

- **File**: `crates/arawn-agent/src/tool/gate.rs` (lines 622-646)
- **Issue**: `MockSecretResolver` struct and `ctx_with_resolver` function are defined in `#[cfg(test)]` but **never called** (compiler warnings confirm: "struct MockSecretResolver is never constructed", "function ctx_with_resolver is never used"). The `validate_tool_paths()` function and `execute_shell_via_sandbox()` function have test scaffolding that was never completed.
- **Risk**: The fs gate is the security boundary preventing the agent from reading/writing files outside allowed paths. Incomplete gate tests mean path traversal attacks may not be caught.

#### 2.7 Tool Registry Output Config and Filtering (`arawn-agent/src/tool/registry.rs`) - Dead test code

- **File**: `crates/arawn-agent/src/tool/registry.rs` (lines 378, 430)
- **Issue**: Functions `test_registry_output_config_for()` and `test_filtered_by_names_includes_matching()` exist in test module but **are never called** (compiler warnings: "function test_registry_output_config_for is never used"). These are test helper functions that were written but the actual `#[test]` functions calling them were deleted or never written.
- **Risk**: Tool filtering controls which tools are exposed to the agent. Incorrect filtering could expose dangerous tools.

#### 2.8 TUI Rendering (`arawn-tui/src/ui/`) - 52.5% coverage, UI modules untested

- **Files with NO tests**: `ui/chat.rs`, `ui/input.rs`, `ui/layout.rs`, `ui/logs.rs`, `ui/sessions.rs`, `ui/tools.rs`, `ui/theme.rs`, `ui/palette.rs`, `ui/mod.rs`, `events.rs`
- **Risk**: While UI bugs are less dangerous than server bugs, the TUI is a primary user interface. Rendering bugs, input handling issues, or event loop problems affect user experience. The `events.rs` module handles terminal event processing and has zero tests.

#### 2.9 Memory Query System (`arawn-memory/src/store/query.rs`) - NO TESTS

- **File**: `crates/arawn-memory/src/store/query.rs`
- **What's untested**: `RecallQuery`, `TimeRange::cutoff()`, `RecallResult` scoring, `SearchResult` ranking
- **Risk**: The query system determines what the agent remembers. Bugs in time range filtering, similarity scoring, or result ranking directly affect AI response quality.

#### 2.10 Server Config (`arawn-server/src/config.rs`) - NO TESTS

- **File**: `crates/arawn-server/src/config.rs`
- **Risk**: Server configuration controls auth tokens, rate limits, CORS, bind addresses. Misconfigured defaults could expose the server without authentication.

### RISK LEVEL: MODERATE

#### 2.11 Concurrency: Session Cache Race Conditions

- **File**: `crates/arawn-session/src/cache.rs`
- **What's tested**: Basic insert/get/evict/TTL operations (single-threaded async)
- **What's NOT tested**: Concurrent access from multiple tasks simultaneously. The cache uses `RwLock` but there are no tests that spawn multiple tasks doing concurrent get_or_load/insert/update/cleanup operations. The `get_or_load` method has a TOCTOU pattern (read lock, drop, write lock, check again) that is untested under contention.

#### 2.12 Concurrency: WebSocket Session Ownership

- **File**: `crates/arawn-server/tests/websocket_ownership.rs` (8 tests)
- **What's tested**: Basic ownership transfer between connections.
- **What's NOT tested**: Rapid ownership contention, connection drops during active chat streams, reconnect token race between two clients.

#### 2.13 Agent Orchestrator (`arawn-agent/src/orchestrator.rs`)

- **What's tested**: Config defaults, basic doc tests
- **What's NOT tested**: The `run()` method (line 160) which manages the explore-compact-continue cycle. This is the core loop that enables agents to work beyond context limits. No tests verify:
  - Compaction triggers at threshold
  - max_compactions safety valve
  - max_turns safety valve
  - Proper session state after compaction

#### 2.14 MCP Client Error Handling (`arawn-mcp/src/client.rs`)

- **What's tested**: Protocol-level tests via integration tests
- **What's NOT tested**: Client reconnection logic, transport failure recovery, timeout handling for stdio child processes, graceful shutdown when MCP server crashes

#### 2.15 Agent Tools (7 tools with no unit tests)

| Tool File | LOC (approx) | Unit Tests |
|-----------|------|------|
| `tools/web.rs` | ~200 | Has #[cfg(test)] with basic tests |
| `tools/file.rs` | ~400 | Has #[cfg(test)] with basic tests |
| `tools/explore.rs` | ~300 | Has #[cfg(test)] with basic tests |
| `tools/delegate.rs` | ~200 | Has #[cfg(test)] with basic tests |
| `tools/note.rs` | ~150 | Has #[cfg(test)] with basic tests |
| `tools/workflow.rs` | ~200 | Has #[cfg(test)] with basic tests |
| `tools/search.rs` | ~200 | Has #[cfg(test)] with basic tests |
| `tools/memory.rs` | ~200 | Has #[cfg(test)] with basic tests |
| `tools/think.rs` | ~50 | Has #[cfg(test)] with basic tests |
| `tools/catalog.rs` | ~150 | Has #[cfg(test)] with basic tests |

Most tool modules have some tests, but they test only happy paths. Error paths (invalid params, backend failures, permission denials) are largely untested.

---

## 3. Recommended Testing Plan

### Priority 1: MUST HAVE before any release

#### P1.1 WebSocket Connection Lifecycle Tests
- **What**: Unit tests for `handle_socket()`, `send_message()`, idle timeout enforcement, authentication during WS upgrade
- **Why**: Primary real-time channel; bugs = data loss or auth bypass
- **Where**: `crates/arawn-server/src/routes/ws/connection.rs`
- **Approach**: Use `tokio-tungstenite` test client. Test: (1) unauthenticated connection rejected, (2) authenticated connection receives messages, (3) idle timeout closes connection after 5 min, (4) cancellation token stops active streams, (5) send_message to closed connection doesn't panic

#### P1.2 Shell Tool Security Tests
- **What**: Tests for command blocklist bypass, sandbox enforcement, output truncation, timeout
- **Why**: Executes arbitrary shell commands; highest attack surface
- **Where**: `crates/arawn-agent/src/tools/shell.rs`
- **Approach**: Test with `MockToolContext`. Verify: (1) each blocked command pattern is rejected, (2) shell metacharacter injection fails (`;`, `&&`, `||`, `` ` ``, `$()`), (3) commands exceeding `max_output_size` are truncated, (4) commands exceeding timeout are killed, (5) PTY mode works for interactive commands, (6) working directory is enforced

#### P1.3 Complete the Filesystem Gate Tests
- **What**: Activate and complete the dead test code in `gate.rs` and `registry.rs`
- **Why**: The fs gate is the security boundary; dead test code means known untested paths
- **Where**: `crates/arawn-agent/src/tool/gate.rs` (lines 622-646), `crates/arawn-agent/src/tool/registry.rs` (lines 378, 430)
- **Approach**: (1) Wire up `MockSecretResolver` tests, (2) test `validate_tool_paths()` for path traversal (`../../../etc/passwd`), (3) test `execute_shell_via_sandbox()` with sandbox errors, (4) complete `test_registry_output_config_for` and `test_filtered_by_names_includes_matching`

#### P1.4 Session Persistence Tests
- **What**: Unit tests for `PersistenceHook` trait, `NoPersistence`, `SessionData`
- **Why**: Data loss on server restart if persistence breaks
- **Where**: `crates/arawn-session/src/persistence.rs`
- **Approach**: (1) Test `SessionData::new()` and builder methods, (2) test `NoPersistence` returns `Ok(None)` for load, `Ok(())` for save/delete, (3) test `on_evict` default implementation

#### P1.5 Server Config Defaults and Validation
- **What**: Unit tests for `ServerConfig` construction and default values
- **Why**: Misconfigured defaults could expose server without auth
- **Where**: `crates/arawn-server/src/config.rs`
- **Approach**: Test: (1) default auth_token is None (localhost mode), (2) rate limiting defaults enabled, (3) all config builder methods work correctly

#### P1.6 Add Coverage Measurement to CI
- **What**: Add `cargo-llvm-cov` or `tarpaulin` to the CI pipeline
- **Why**: Coverage data exists locally for only 12/20 crates and is not tracked over time. Six of the largest crates have ZERO coverage data.
- **Where**: `.github/workflows/ci.yml`
- **Approach**: Add a coverage job using `cargo-llvm-cov` with `--workspace` flag, upload to Codecov/Coveralls, set minimum thresholds (e.g., 70% per crate, no decrease on PR)

### Priority 2: SHOULD HAVE

#### P2.1 Concurrent Session Cache Tests
- **What**: Multi-task stress tests for `SessionCache`
- **Why**: TOCTOU race in `get_or_load()`, concurrent eviction during insert
- **Where**: `crates/arawn-session/src/cache.rs`
- **Approach**: Spawn 50 tokio tasks doing concurrent get_or_load/insert/update. Use `loom` or manual synchronization testing. Verify no panics, no lost updates, LRU ordering maintained.

#### P2.2 CLI Command Unit Tests
- **What**: Unit tests for each command module in `crates/arawn/src/commands/`
- **Why**: 23.9% coverage; user-facing entry points
- **Where**: `crates/arawn/src/commands/` (17 files)
- **Approach**: For each command: (1) test argument parsing/validation, (2) test output formatting for JSON and human modes, (3) test error handling for common failure modes (server unreachable, 401/403/404/500)
- **Specific priority**: `start.rs` (server startup), `auth.rs` (authentication flow), `secrets.rs` (secret management), `config.rs` (config manipulation)

#### P2.3 OAuth Passthrough Forwarding Tests
- **What**: Tests for `forward_raw()` and `forward_raw_stream()`
- **Why**: Forwards authenticated requests; bugs could leak credentials
- **Where**: `crates/arawn-oauth/src/passthrough.rs`
- **Approach**: Use `wiremock` or `mockito` to create a mock upstream. Test: (1) Bearer token is injected, (2) streaming response is proxied correctly, (3) upstream errors are translated to proper error responses, (4) API key extraction from headers works for all cases

#### P2.4 Agent Orchestrator Integration Tests
- **What**: Tests for the `CompactionOrchestrator::run()` method
- **Why**: Core loop for long-running agent tasks; bugs = infinite loops or truncated results
- **Where**: `crates/arawn-agent/src/orchestrator.rs`
- **Approach**: Use `ScriptedMockBackend` to simulate: (1) context growth triggering compaction, (2) max_compactions limit enforced, (3) max_turns limit enforced, (4) session state preserved across compaction cycles

#### P2.5 Memory Query System Tests
- **What**: Unit tests for `RecallQuery`, `TimeRange`, result scoring
- **Why**: Determines what the agent remembers; bugs = wrong context
- **Where**: `crates/arawn-memory/src/store/query.rs`
- **Approach**: Test: (1) `TimeRange::cutoff()` for each variant, (2) `RecallQuery` with various embedding sizes, (3) result ordering and limit enforcement

#### P2.6 MCP Client Resilience Tests
- **What**: Tests for transport failures, reconnection, timeout
- **Why**: MCP servers are external processes that can crash
- **Where**: `crates/arawn-mcp/src/client.rs`
- **Approach**: Use mock transport. Test: (1) timeout on unresponsive server, (2) graceful handling of EOF on stdio, (3) HTTP retry logic, (4) initialize handshake failure recovery

#### P2.7 TUI Event Handling Tests
- **What**: Tests for `events.rs` event loop and terminal event processing
- **Why**: Primary interactive interface; event bugs = frozen UI
- **Where**: `crates/arawn-tui/src/events.rs`
- **Approach**: Mock the crossterm event stream. Test: (1) key events route to correct handlers, (2) resize events update layout, (3) paste events are handled, (4) ctrl+c triggers shutdown

#### P2.8 Error Path Coverage for All Tools
- **What**: Error path tests for each tool in `crates/arawn-agent/src/tools/`
- **Why**: Tools currently test only happy paths
- **Where**: All tool files
- **Approach**: For each tool, test: (1) missing required parameters, (2) invalid parameter types, (3) backend/service failure, (4) permission denied, (5) timeout

### Priority 3: NICE TO HAVE

#### P3.1 Property-Based Tests for Security-Critical Paths
- **What**: Extend proptest coverage beyond `path_validator` to other security boundaries
- **Why**: Fuzzing finds edge cases humans miss
- **Where**: `arawn-agent/src/tool/gate.rs`, `arawn-agent/src/tools/shell.rs`, `arawn-config/src/secret_store.rs`, `arawn-server/src/auth.rs`
- **Approach**: Add `proptest` dependency. Generate: (1) arbitrary file paths for gate validation, (2) arbitrary shell commands for blocklist testing, (3) arbitrary auth tokens for constant-time comparison, (4) arbitrary secret names/values for round-trip testing

#### P3.2 Snapshot Tests for TUI Rendering
- **What**: Use `insta` or `ratatui-test` for TUI widget snapshot testing
- **Why**: Catch rendering regressions without manual visual inspection
- **Where**: `crates/arawn-tui/src/ui/`
- **Approach**: Render each widget to a `Buffer`, snapshot the output, compare in CI

#### P3.3 Load/Stress Tests for Server
- **What**: Extend `e2e_stress.rs` with higher concurrency and longer runs
- **Why**: Current stress tests are modest; production loads may be higher
- **Where**: `crates/arawn-server/tests/e2e_stress.rs`
- **Approach**: (1) 100+ concurrent WebSocket connections, (2) rapid message sending, (3) measure memory growth over time, (4) verify no connection leaks

#### P3.4 Pipeline Runtime Integration Tests
- **What**: Tests for all 6 runtimes (file_read, file_write, http, shell, transform, passthrough)
- **Why**: Each runtime has `#[cfg(test)]` modules but they're minimal
- **Where**: `runtimes/*/src/main.rs`
- **Approach**: Test each runtime's IPC protocol (stdin/stdout JSON-RPC) with mock inputs

#### P3.5 Backward Compatibility Tests for Config/Data Formats
- **What**: Tests that verify old config files and data formats can be loaded by new code
- **Why**: Users upgrade; breaking changes in serialization formats cause data loss
- **Where**: `arawn-config/src/types.rs`, `arawn-workstream/src/store.rs`, `arawn-memory/src/store/`
- **Approach**: Store fixture files of old format versions and test deserialization

#### P3.6 Audit gline-rs-vendored (NER Engine)
- **What**: The vendored NER engine has 0 tests in the current test suite
- **Why**: It's responsible for entity extraction which feeds the knowledge graph
- **Where**: `crates/gline-rs-vendored/`
- **Approach**: Port any upstream tests; add integration tests for entity extraction quality

---

## 4. Testing Infrastructure Recommendations

### 4.1 CI/CD Pipeline

| Item | Current State | Recommendation |
|------|--------------|----------------|
| Coverage in CI | Not present | Add `cargo-llvm-cov` job with per-crate reporting and minimum threshold (70%) |
| Coverage for ALL crates | 6 largest crates missing | Ensure `--workspace` flag covers agent, server, llm, oauth, plugin, workstream |
| Coverage trend | No tracking | Upload to Codecov/Coveralls; block PRs that decrease coverage |
| Test parallelism | `--test-threads=1` (serialized) | Investigate which tests need serialization vs which can run in parallel. Separate serial tests into a `#[serial_test::serial]` group. |
| Ignored tests | 109 doc tests always ignored | Move doc tests that need backends to integration tests; enable doc tests that are self-contained |

### 4.2 Test Harness Improvements

| Item | Recommendation |
|------|----------------|
| Dead test code | Fix compiler warnings: 5 unused test functions in arawn-agent (gate.rs:622-646, registry.rs:378,430). Either complete them or delete them. |
| Mock improvements | Add a `MockFsGate` to arawn-test-utils for testing tool path validation without filesystem |
| Sandbox testing in CI | CI runs on Ubuntu; add bubblewrap/socat to CI image so sandbox tests don't self-skip |
| Property testing | Add `proptest` to workspace dependencies; use for security boundaries |
| Test organization | Some integration test files are very large (e.g., `command_integration.rs` at 43 tests). Consider splitting by command category. |
| Flaky test detection | Add `cargo-nextest` for better test retry and timing output |

### 4.3 Performance/Load Testing

| Area | Need |
|------|------|
| WebSocket throughput | Measure max messages/sec per connection; validate backpressure |
| Session cache under load | Measure latency of get_or_load under 100+ concurrent requests |
| Memory store query performance | Benchmark vector similarity search at 10K, 100K, 1M memories |
| Agent turn latency | Measure overhead of tool validation, fs gate checks, output processing |
| Startup time | Measure cold start of server with various config sizes |

### 4.4 Security Testing

| Area | Need |
|------|------|
| Auth bypass | Fuzz the auth middleware with malformed Authorization headers |
| Path traversal | Fuzz fs_gate with symlinks, `.`, `..`, null bytes, unicode normalization |
| Shell injection | Fuzz shell tool with encoded characters, ANSI escape sequences |
| Rate limit bypass | Test with spoofed X-Forwarded-For headers (IP header injection) |
| Secret exposure | Verify Debug/Display impls never leak secret values (test already exists for AgeSecretStore; extend to all types holding secrets) |
| Constant-time auth | Existing tests verify correctness but not timing properties. Consider a timing-based test or audit. |

---

## 5. Appendix: Files With ZERO Test Coverage

The following source files have NO `#[cfg(test)]` module AND are not covered by integration tests:

### Critical (security/data)
- `crates/arawn-server/src/routes/ws/connection.rs` - WebSocket lifecycle
- `crates/arawn-server/src/config.rs` - Server configuration
- `crates/arawn-session/src/persistence.rs` - Session persistence hooks

### High (user-facing)
- `crates/arawn/src/commands/ask.rs` - Ask command
- `crates/arawn/src/commands/chat.rs` - Chat command
- `crates/arawn/src/commands/agent.rs` - Agent command
- `crates/arawn/src/commands/start.rs` - Server start command
- `crates/arawn/src/commands/auth.rs` - Auth command
- `crates/arawn/src/commands/secrets.rs` - Secrets command
- `crates/arawn/src/commands/session.rs` - Session command
- `crates/arawn/src/commands/mcp.rs` - MCP command
- `crates/arawn/src/commands/memory.rs` - Memory command
- `crates/arawn/src/commands/notes.rs` - Notes command
- `crates/arawn/src/commands/repl.rs` - REPL command
- `crates/arawn/src/commands/plugin.rs` - Plugin command
- `crates/arawn/src/commands/status.rs` - Status command
- `crates/arawn/src/commands/tui.rs` - TUI launcher
- `crates/arawn/src/commands/config.rs` - Config command
- `crates/arawn/src/commands/logs.rs` - Logs command
- `crates/arawn/src/commands/output.rs` - Output formatting

### Moderate (infrastructure)
- `crates/arawn-server/src/routes/openapi.rs` - OpenAPI spec generation
- `crates/arawn-server/src/routes/logs.rs` - Log viewing routes
- `crates/arawn-memory/src/store/query.rs` - Memory query types
- `crates/arawn-tui/src/events.rs` - Terminal event processing
- `crates/arawn-tui/src/ui/chat.rs` - Chat rendering
- `crates/arawn-tui/src/ui/input.rs` - Input rendering
- `crates/arawn-tui/src/ui/layout.rs` - Layout rendering
- `crates/arawn-tui/src/ui/logs.rs` - Log rendering
- `crates/arawn-tui/src/ui/sessions.rs` - Session list rendering
- `crates/arawn-tui/src/ui/tools.rs` - Tool output rendering
- `crates/arawn-tui/src/ui/theme.rs` - Theme definitions
- `crates/arawn-tui/src/ui/palette.rs` - Command palette rendering
- `crates/arawn-client/src/api/*.rs` (13 files) - Client API methods (covered by integration tests)
- `crates/arawn-agent/src/rlm/types.rs` - RLM types
- `crates/arawn-agent/src/rlm/prompt.rs` - RLM prompts
- `crates/arawn-agent/src/indexing/gliner.rs` - GLiNER integration
- `crates/gline-rs-vendored/` - Entire vendored NER crate (0 tests)

### Low (error types, lib.rs re-exports)
- Various `error.rs`, `lib.rs`, `mod.rs` files across all crates (typically just re-exports or error enum definitions)