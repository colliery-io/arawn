# Correctness Review

## Summary

The arawn codebase demonstrates strong correctness practices for a project at this stage. The test infrastructure is well-designed -- the `TestHarness` builder makes it easy to compose subsystems into realistic integration scenarios, and the mock LLM client supports streaming deltas for faithful simulation. Error handling is consistent across crates (typed errors via `thiserror`, user-facing messages, circuit breakers). The main correctness risks cluster around concurrent session access (no guard against parallel `send_message` to the same session), silent fallback on malformed LLM arguments, a security-relevant permission bypass via session grants, and unsafe code in the embedding layer.

## Test Coverage Assessment

### Strengths

- **Integration test breadth**: 13 integration test files covering the full pipeline, compaction, persistence, hooks, hot-reload, local service, memory, permissions, plugins, skills, websocket, and workflows. Tests exercise real subsystem interactions, not just mocks.
- **Behavioral tests**: Tests focus on what happens, not how. For example, permission tests verify that denied tools produce error `ToolResult` messages, not that specific internal methods were called.
- **Edge case coverage**: Stream without `Done` chunk, empty stream, split JSON arguments across deltas, parallel tool calls in single turn, mixed text and tool calls -- all tested.
- **Test infrastructure quality**: `TestHarness` builder with `with_workstream_file()`, `with_permission_checker()`, `with_hook_runner()`, `with_skill_registry()`, `with_plan_active()`, `with_progress_channel()` -- composable and production-realistic.
- **Inline unit tests**: Every major module has a `#[cfg(test)] mod tests` block with focused unit tests (session, JSONL, permissions, rules, compactor, vector, tool_result_limiter, database).

### Gaps

- **No concurrent access tests**: No tests exercise parallel `send_message` calls to the same session. The single-threaded test runner (`--test-threads=1`) masks potential race conditions.
- **No cancellation tests**: `cancel()` is a no-op, so there are no tests for cancellation mid-stream, but the lack of cancellation itself is a correctness gap (engine runs unbounded until `max_iterations`).
- **No malformed JSONL tests**: No test for corrupted or partial JSONL lines (e.g., crash during append). The `load()` path would fail on `serde_json::from_str` with no recovery.
- **No test for `parse_arguments` fallback**: The `parse_arguments()` function silently returns `{}` on any parse failure, but no test verifies this behavior or validates that tools handle receiving empty arguments gracefully.
- **Limited error path coverage for tools**: Individual tool `execute()` error paths (e.g., file_edit on nonexistent file, shell timeout, web_fetch network failure) are not systematically exercised through the engine integration path.

## Key Risk Areas

### 1. Concurrent Session Access (High Risk)

`send_message()` in `local_service.rs` (line 548) loads the session from JSONL, runs the engine, then appends new messages. There is no lock or guard preventing two concurrent `send_message` calls for the same `session_id`. Two parallel calls would:
1. Both load the same JSONL messages
2. Both run the engine independently
3. Both append their messages to the same JSONL file

This would produce interleaved messages in the JSONL file, corrupting the session history. The WebSocket handler in `ws_server.rs` processes messages in a loop, but nothing prevents a second WebSocket connection (or a CLI client) from hitting `send_message` concurrently.

### 2. Session Grant Overrides Deny Rules (Medium Risk)

In `permissions/checker.rs` (line 232), the `check()` method short-circuits on session grants *before* evaluating rules. This means if a user selects "Allow Always" for tool X, and then permission rules are hot-reloaded to add a `Deny` rule for tool X, the deny rule is bypassed. The comment at line 231 documents the intended priority ("deny > allow > ask > no match") but the session grant check at line 234 runs first and overrides everything. This is visible in the test at line 512-519 which explicitly demonstrates this behavior -- a session grant bypasses even a deny rule.

### 3. Silent Argument Fallback (Medium Risk)

`parse_arguments()` at `query_engine.rs:843-848` silently converts any invalid JSON to `{}`:
```rust
fn parse_arguments(raw: &str) -> serde_json::Value {
    if raw.is_empty() {
        return serde_json::json!({});
    }
    serde_json::from_str(raw).unwrap_or(serde_json::json!({}))
}
```
When an LLM produces malformed argument JSON (common with smaller models), the tool receives empty arguments, fails with a potentially confusing error, and the `failed_call_counts` circuit breaker may block subsequent calls. The parse failure is not logged or surfaced.

### 4. Promotion Inconsistency Window (Medium Risk)

In `store.rs:203-256`, `promote_session()` updates SQLite first, then moves the JSONL file. If the file move fails (e.g., cross-device rename on Linux), the code logs a warning and returns an error, but the SQLite update has already committed -- the session metadata says it belongs to a workstream but the JSONL file is still in the scratch location. The split `promote_session_metadata` + `move_session_jsonl` in `local_service.rs` has the same issue.

## Findings

### COR-001: No Guard Against Concurrent Session Access
- **Severity**: High
- **Location**: `crates/arawn/src/local_service.rs:548-820`
- **Description**: `send_message()` loads session state, runs the engine, and persists results without any per-session lock. Concurrent calls for the same session will corrupt the JSONL message file with interleaved writes. The spawned task (line 720) operates on a snapshot of the session, so two tasks both append to the same file without coordination.
- **Impact**: Message corruption, duplicate tool executions, inconsistent session state.
- **Recommendation**: Add a per-session lock (e.g., `Arc<Mutex<()>>` keyed by session ID in a `DashMap`) acquired before loading and released after persisting. Reject or queue concurrent sends to the same session.

### COR-002: Session Grants Bypass Explicit Deny Rules
- **Severity**: Medium
- **Location**: `crates/arawn-engine/src/permissions/checker.rs:232-237`
- **Description**: Session grants short-circuit the entire permission check, including deny rules. If a user clicks "Allow Always" for a tool, no subsequently added deny rule will block it until the session grants are cleared. This is explicitly tested (line 512) but represents a security-relevant design choice that may surprise operators who add deny rules expecting them to be absolute.
- **Impact**: Deny rules can be inadvertently bypassed; hot-reloaded deny rules do not take effect for already-granted tools.
- **Recommendation**: Move the session grant check to after deny rule evaluation: deny should always win.

### COR-003: Silent parse_arguments Fallback Masks LLM Failures
- **Severity**: Medium
- **Location**: `crates/arawn-engine/src/query_engine.rs:843-848`
- **Description**: When the LLM produces malformed JSON for tool arguments (truncated, invalid escapes, etc.), `parse_arguments()` silently returns `{}`. The tool then runs with empty arguments and likely fails, but the root cause (bad JSON from LLM) is invisible. There is also a validation check at line 366-376 that rejects non-object arguments, but by the time `parse_arguments` runs, the malformed JSON has already been silently converted to `{}` (which is a valid object).
- **Impact**: Tool failures with unhelpful error messages; the circuit breaker may block future valid calls to the same tool.
- **Recommendation**: Log a warning when JSON parsing fails with the raw string. Consider returning a sentinel error value rather than `{}` so the argument validation at line 366 can catch it.

### COR-004: Promotion Non-Atomicity Creates Inconsistent State
- **Severity**: Medium
- **Location**: `crates/arawn-storage/src/store.rs:203-256`
- **Description**: Session promotion updates SQLite metadata and then moves the JSONL file in separate operations. If the file move fails, the session's SQLite record points to a workstream but the JSONL file remains in the scratch directory. The `local_service.rs:369-416` version has the same two-phase issue but uses a separate `JsonlMessageStore` instance, which doesn't share any transactional boundary with the locked store.
- **Impact**: Orphaned sessions that cannot load (no JSONL at expected path) but appear in the workstream's session list.
- **Recommendation**: Either move the file first (easier to undo if SQLite update fails) or implement a cleanup/recovery check that detects this inconsistency on startup.

### COR-005: `truncate_input` Panics on Multi-Byte Characters
- **Severity**: Low
- **Location**: `crates/arawn-engine/src/permissions/checker.rs:316-322`
- **Description**: `truncate_input()` slices the input string at byte offset `max_len` with `&input[..max_len]`. If `max_len` falls in the middle of a multi-byte UTF-8 character, this will panic. The tool_result_limiter correctly handles this (line 64-68 searches for a char boundary), but the permission module does not.
- **Impact**: Panic on non-ASCII tool input content longer than 200 chars (e.g., file paths with Unicode characters, or commands with emoji).
- **Recommendation**: Use the same char-boundary search pattern as `tool_result_limiter.rs:64-68`.

### COR-006: `unsafe impl Send/Sync` for LocalEmbedder
- **Severity**: Low
- **Location**: `crates/arawn-embed/src/local.rs:33-34`
- **Description**: `LocalEmbedder` manually implements `Send` and `Sync` via unsafe with the justification that `Mutex<Session>` serializes access and `Tokenizer` is Send+Sync. However, the ORT `Session` type is `!Send` for a reason (it may use thread-local GPU state). The safety comment assumes CPU-only execution, but this is not enforced -- if ORT is configured with CUDA or other GPU providers, the unsafe Send impl could cause undefined behavior.
- **Impact**: Potential UB if ORT is configured with non-thread-safe execution providers.
- **Recommendation**: Add a runtime check or configuration guard that ensures only CPU execution provider is used, or pin execution to a single thread.

### COR-007: `filter_tools_for_context` May Hide Necessary Tools
- **Severity**: Low
- **Location**: `crates/arawn-engine/src/query_engine.rs:895-1003`
- **Description**: After the first 2 messages, tool definitions sent to the LLM are filtered by keyword heuristics on the last user message. Tools not in the core set (e.g., `workstream_create`, `workstream_list`, `memory_store`, `memory_search`, `Agent`) require specific keywords to be included. If the user says "set up a new project workspace" (no keyword "workstream"), the workstream tools won't be included. Workstream tools are notably absent from all the filter categories.
- **Impact**: LLM cannot use certain tools when the user's language doesn't match the hardcoded keyword patterns.
- **Recommendation**: Add workstream tools to a filter category, or consider making the filtering more conservative (include more tools by default, only filter out truly niche ones).

### COR-008: JSONL Append Not Crash-Safe
- **Severity**: Low
- **Location**: `crates/arawn-storage/src/jsonl.rs:28-51`
- **Description**: The JSONL append operation opens the file, writes a JSON line, but does not call `flush()` or `fsync()`. A crash between write and OS flush could result in a partial line at the end of the JSONL file. On reload, `serde_json::from_str` would fail on the partial line, and the error propagates as `StorageError`, making the entire session unloadable.
- **Impact**: Session data loss after crash -- all messages including non-corrupt ones become inaccessible.
- **Recommendation**: Either `fsync()` after each append (performance cost) or add error recovery in `load()` that skips/truncates the last corrupt line (more practical).

### COR-009: Compactor Uses Same LLM Client for Main and Compaction
- **Severity**: Low  
- **Location**: `crates/arawn/src/local_service.rs:666`
- **Description**: The compactor is constructed with `self.llm.clone()`, sharing the same LLM client as the main engine. The config mentions an optional separate LLM for compaction (`[compactor]` section in config), but `local_service.rs` always uses the main LLM. If the main LLM is rate-limited or has a small context window, compaction calls compete for the same quota and may fail, triggering the circuit breaker.
- **Impact**: Compaction and main query compete for LLM rate limits; config option for separate compaction LLM is unused.
- **Recommendation**: Wire in the separate compaction LLM from config when specified.

### COR-010: `failed_call_counts` Never Resets Between Messages
- **Severity**: Low
- **Location**: `crates/arawn-engine/src/query_engine.rs:106`
- **Description**: The `failed_call_counts` HashMap is per-`QueryEngine` instance. Since `send_message` creates a new `QueryEngine` for each message (line 667), this map is always empty at the start of each user turn. The circuit breaker only protects against repeated failures within a single agentic loop run, not across messages. This is correct behavior (the engine is stateless between messages) but means a tool call that fails identically in two separate user messages will never be blocked by this mechanism.
- **Impact**: Minimal -- the intended behavior (within-turn dedup) works correctly. Cross-turn dedup would require shared state.
