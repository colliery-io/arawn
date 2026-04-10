# Operability Review

## Summary

Arawn has a solid operational foundation for a local-first tool. Configuration is well-structured with sensible defaults, environment variable overrides, hot-reload via file watchers, and graceful fallback when the config file is missing or malformed. Logging is present throughout using the `tracing` crate with dual-output (file + stderr in serve mode, file-only in TUI mode, stderr-only in CLI mode). LLM errors have user-facing messages with actionable guidance, and transient failures are retried with exponential backoff.

The gaps are concentrated in three areas: (1) no signal handling or graceful shutdown for the main server process, (2) no metrics, health checks, or process status reporting of any kind, and (3) disk-full / storage failure paths that silently swallow errors in the spawned engine task. These are all reasonable omissions for the current stage of the project, but several will bite a daily-driver user who encounters them.

**Overall assessment: Adequate with notable gaps.** The system handles the happy path well and degrades gracefully for LLM failures. It does not handle infrastructure failures (disk, process signals, port conflicts) robustly, and provides no observability beyond log files.

## Observability Assessment

### Logging

**Strengths:**
- Dual logging in serve mode: rolling daily file (`~/.arawn/logs/server.log`) at debug level for arawn crates, stderr at info level (or `RUST_LOG` override). TUI mode logs only to file, keeping terminal clean.
- Log filter string (`FILE_LOG_FILTER`) explicitly sets debug for all arawn crates and warn for third-party, which is the right default.
- Key lifecycle events are logged: store open, config load, LLM provider selection, tool registration count, session create, MCP connections, memory initialization, workflow runner start, config hot-reload.
- WebSocket handler logs connection lifecycle, RPC dispatch, and error paths with request IDs.
- `tracing_appender::non_blocking` with `WorkerGuard` ensures logs flush on exit.

**Gaps:**
- Zero use of `#[instrument]` anywhere in the codebase. Every function manually constructs its own debug/info/warn calls. This means no automatic span context, no timing, and no structured field propagation across async boundaries.
- No request/session ID propagation via tracing spans. The WebSocket handler logs `id` (the RPC request ID), but the spawned engine task and all tool executions have no span linking them back to the originating request.
- API keys are passed through `format!("Bearer {}", self.api_key)` in the Groq and Anthropic clients. While not logged directly, the `api_key` field is a plain `String` on the client struct with no redaction wrapper -- a `Debug` derive on any struct containing the client would leak it.

### Metrics

None. No counters, histograms, or gauges anywhere. No `prometheus`, `metrics`, or custom tracking beyond `SessionStats` (which is per-session cumulative tokens/turns stored in SQLite).

For a local-first tool, full metrics infrastructure is not warranted, but basic operational counters would aid debugging: total requests served, active sessions, LLM call latency, retry counts, tool execution counts, compaction frequency.

### Tracing (Distributed/Request)

None. No span context, no trace IDs, no correlation between the WebSocket request, the engine loop iterations, and the individual tool executions. Each log line is independent.

### Health and Readiness

None. No health endpoint on the HTTP server. No readiness check. No way to determine if the server is running, healthy, or accepting connections other than attempting a WebSocket connection. The CLI mode's error message when the server is unreachable ("Could not connect to arawn server ... Start the server first: arawn serve") is the only diagnostic.

## Failure Mode Analysis

### LLM API Failures (Well Handled)

- **Wrong API key:** `LlmError::Auth` with `user_message()` that says "check that your API key is set correctly". Not retried (`is_retryable()` returns false for Auth). Surfaced to user via `EngineEvent::Error`.
- **Rate limiting:** `LlmError::RateLimited` is retryable. `RetryClient` retries up to 3 times with exponential backoff (1s, 2s, 4s). User-facing message explains retry behavior.
- **Server errors (5xx):** Retryable with same backoff strategy.
- **Model not found:** Non-retryable, user message points to `arawn.toml`.
- **Network failures:** Timeout and connection errors are retryable; user message differentiates timeout vs. connection failure.

This is well-engineered. The `user_message()` pattern on both `LlmError` and `EngineError` gives actionable guidance without exposing internals.

### Configuration Failures (Well Handled)

- **Missing arawn.toml:** Falls back to defaults, logs at debug level. Generates a default config file on first startup.
- **Malformed arawn.toml:** Logs a warning with the parse error, falls back to full defaults. Does not partially apply a broken config.
- **Missing API key env var:** `build_llm_client()` in `main.rs` returns an error before the server starts, with a clear message naming the expected variable.
- **Referenced LLM config doesn't exist:** `engine_llm()` falls back to "default" config, which is always ensured to exist by `load()`. No panic possible.

### Storage Failures (Partially Handled)

- **SQLite locked/busy:** WAL mode + 5000ms busy timeout configured in `Database::open()`. This handles brief contention but a truly locked database (e.g., another process holding an exclusive lock) will error after 5 seconds.
- **Disk full during JSONL append:** The `append()` call in the spawned engine task (local_service.rs:785) uses `if let Err(e)` and logs with `error!` but continues. The session's messages are lost silently -- no `EngineEvent::Error` is emitted for individual message persistence failures. The `Complete` event is still sent with `final_text`, so the user sees a normal response but their conversation history is corrupted.
- **Disk full during SQLite update:** Stats update (local_service.rs:792) uses `let _ = s.update_session_stats(...)`, completely silencing any error. No log, no event.
- **Corrupted JSONL:** `serde_json::from_str` on each line returns a `StorageError::Json` if any line is malformed. There is no line-skip or recovery -- the entire session load fails. If a crash occurred mid-append, the partial line would make the session unloadable.

### Port Conflicts (Not Handled)

- `TcpListener::bind()` returns an `Err` which propagates via `?` and prints a generic anyhow error. The error message from the OS ("Address already in use") is technically sufficient but there's no check for whether another arawn instance is already running, and no suggestion to use `--port`.

### Process Lifecycle (Not Handled)

- **No signal handling:** The server relies entirely on `axum::serve()`, which uses the default tokio behavior of aborting on Ctrl-C. There is no `tokio::signal::ctrl_c()` handler, no `SIGTERM` handler, no graceful drain of in-flight requests. The workflow runner's `shutdown()` method exists (lines 425-428 in main.rs) but it's placed *after* `axum::serve().await`, which means it only runs after the server has already stopped -- if the server was killed by a signal, it never executes.
- **No cleanup of background tasks:** `BackgroundTaskManager` tracks running tasks but has no shutdown method called on exit. Tasks spawned by the shell tool or agent tool continue as orphans.
- **WebSocket connections not drained:** Active streaming sessions are abruptly disconnected. No "server shutting down" message is sent.

### Memory System Failures (Well Handled)

- `try_open_memory()` wraps `MemoryManager::open()` and returns `None` on failure, with a warning log. The server continues without memory features.
- Vector storage initialization failures are logged but don't prevent the rest of the memory system from working.
- The entire memory system is optional -- `Option<Arc<MemoryManager>>` throughout the service layer.

## Findings

### OPS-01: No Signal Handling or Graceful Shutdown (Severity: Medium)

**Location:** `crates/arawn/src/main.rs:423-430`, `crates/arawn/src/ws_server.rs:84`

The server process has no signal handling. `axum::serve(listener, app).await?` blocks until the process is killed. The graceful shutdown code for the workflow runner (main.rs:425-428) is unreachable in a signal-kill scenario because `axum::serve` is aborted, not returned from.

This means:
- In-flight engine loops are killed mid-execution, potentially leaving partial JSONL writes
- Background tasks spawned by the shell/agent tools become orphaned
- The workflow runner's cloacina pipelines are not drained
- No cleanup of temporary files or sandbox directories

**Recommendation:** Add `axum::serve(...).with_graceful_shutdown(shutdown_signal())` where `shutdown_signal()` listens for SIGTERM/SIGINT. On shutdown: cancel active engine tasks (requires implementing the existing `cancel()` TODO), stop background tasks, drain the workflow runner, then exit.

---

### OPS-02: JSONL Persistence Errors Silently Swallowed (Severity: Medium)

**Location:** `crates/arawn/src/local_service.rs:784-788, 791-793`

When the engine completes successfully, new messages are persisted in a loop:

```rust
for msg in &session.messages()[msgs_before..] {
    if let Err(e) = msg_store.append(session_id, &ws_dir_owned, msg).await {
        error!(error = %e, "failed to persist message");
    }
}
```

If disk is full or JSONL write fails, each message is logged as an error but the loop continues. `EngineEvent::Complete` is still emitted. The user has no indication that their conversation was not saved. On the error path (lines 803-804), the same pattern is used with `let _ =`, completely silencing persistence failures.

Similarly, `update_session_stats` (line 792) discards errors with `let _ =`.

**Recommendation:** If any message persistence fails, emit a warning event (e.g., `EngineEvent::Error { message: "Some messages could not be saved..." }`) before the `Complete` event so the user knows their conversation may not survive a restart.

---

### OPS-03: No Span Context or Request Tracing (Severity: Low)

**Location:** All crates -- zero uses of `#[instrument]`

The codebase uses `tracing` (debug!, info!, warn!, error!) extensively, which is good. However, there is no span context anywhere. A typical request flows through:

1. WebSocket handler (ws_server.rs) -- logs RPC `id`
2. `LocalService::send_message()` -- spawns a new tokio task
3. Engine loop iterations -- logs `iteration` number
4. Tool executions -- logs tool name
5. LLM API calls -- logs retry attempts

Each of these logs independently. When debugging "why did session X fail?", the operator must manually correlate by timestamp. Adding `#[instrument(skip_all, fields(session_id = %session_id))]` to `send_message` and key engine methods would automatically propagate session context through all nested logs.

**Recommendation:** Add `#[instrument]` to the top-level request handlers (`send_message`, `load_session`, `create_session`) with `session_id` as a field. This provides automatic span nesting through all downstream tracing calls without changing any existing log statements.

---

### OPS-04: No Health Endpoint (Severity: Low)

**Location:** `crates/arawn/src/ws_server.rs:72-86`

The HTTP server exposes only `/ws` (WebSocket) and `/api/decision` (workflow). There is no `/health` or `/status` endpoint. The TUI and CLI modes detect server availability by attempting a WebSocket connection and reporting failure, but there's no lightweight way to check if the server is running and healthy.

For a local tool, this matters when:
- The user starts the TUI and the server isn't running (current error is adequate)
- Process monitors or scripts want to verify the server is up
- Debugging whether the server started correctly

**Recommendation:** Add a `GET /health` endpoint returning `200 OK` with basic info (uptime, session count, connected clients). This is minimal effort with axum and immediately useful for scripting.

---

### OPS-05: No Log Rotation Limits (Severity: Low)

**Location:** `crates/arawn/src/main.rs:106-107`

Log files use `tracing_appender::rolling::daily`, which creates a new log file each day. However, there is no maximum file count or size limit. Over months of daily use, the `~/.arawn/logs/` directory will accumulate log files indefinitely.

For a local tool that runs daily, this means unbounded disk growth in the data directory. A year of use would produce 365+ log files at debug level, potentially gigabytes.

**Recommendation:** Use `tracing_appender::rolling::RollingFileAppender` with a max file count, or add periodic cleanup logic that removes logs older than N days.

---

### OPS-06: Partial JSONL Write Creates Unrecoverable Session (Severity: Medium)

**Location:** `crates/arawn-storage/src/jsonl.rs:28-50`, `crates/arawn-storage/src/jsonl.rs:54-78`

The JSONL append writes a single line per message:
```rust
let mut line = serde_json::to_string(msg)?;
line.push('\n');
file.write_all(line.as_bytes()).await?;
```

If the process is killed (OPS-01) between serializing and completing the write, a partial JSON line may be left in the file. The `load()` method parses each line with `serde_json::from_str`, which will fail on the partial line, causing `StorageError::Json`. This makes the entire session unloadable -- all messages are lost, not just the corrupted one.

**Recommendation:** Add a recovery mode in `load()` that skips lines which fail to parse (logging a warning for each). This is standard practice for append-only log formats. Alternatively, use `fsync` after each write to reduce the window for partial writes (at a small performance cost).

---

### OPS-07: Config Hot-Reload Does Not Update LLM Client or Engine Settings (Severity: Low)

**Location:** `crates/arawn/src/config_watcher.rs:116-145`

The config watcher reloads permissions and MCP servers when `arawn.toml` changes, but it does not update:
- The LLM client (model name, provider, API key env var, context window, max tokens)
- Engine settings (max_iterations, max_result_size)
- Compactor settings (threshold, keep_recent)
- Server settings (port -- expected, can't change while running)

The log message at line 141 logs the new model name and max_iterations, creating the impression that these were applied, but they are read from the freshly parsed config and immediately discarded.

**Recommendation:** Either extend hot-reload to update the engine config and LLM client (requires making them `Arc<RwLock<>>` or similar), or remove the misleading log line that implies these values were applied. At minimum, log a note that "LLM and engine settings require restart."

---

### OPS-08: No Startup Validation of API Key (Severity: Low)

**Location:** `crates/arawn/src/main.rs:213-215, 587-608`

The server validates that the API key environment variable *exists* at startup (`build_llm_client()` checks `std::env::var`), but does not validate that the key is *valid*. The first indication of a bad key is when the user sends a message and gets an `LlmError::Auth` error after the full engine setup.

For a local tool where startup happens infrequently, this is a minor issue. But when the user starts the server, waits, opens the TUI, creates a session, types a prompt, and *then* gets "Authentication failed", the feedback loop is unnecessarily long.

**Recommendation:** Add an optional lightweight API validation call at startup (e.g., a models list request or a minimal completion). If it fails with Auth, warn on stderr immediately. Make it opt-out via config for offline/air-gapped setups.

---

### OPS-09: Server Port Conflict Produces Opaque Error (Severity: Low)

**Location:** `crates/arawn/src/ws_server.rs:79`

If port 3100 is already in use (e.g., another arawn instance), `TcpListener::bind()` fails with the OS error "Address already in use", which propagates as an anyhow error to stderr. There is no check for whether another arawn process owns the port, and no suggestion to use `--port`.

**Recommendation:** Catch the `AddrInUse` error specifically and print a more helpful message: "Port {port} is already in use. Another arawn instance may be running. Use --port to specify a different port."

---

### OPS-10: Cancellation is Unimplemented (Severity: Medium)

**Location:** `crates/arawn/src/local_service.rs:822-826`

```rust
async fn cancel(&self, _session_id: Uuid) -> Result<(), ServiceError> {
    // TODO: implement cancellation via CancellationToken
    debug!("cancel requested (not yet implemented)");
    Ok(())
}
```

The cancel RPC exists in the WebSocket protocol, the TUI sends it (Ctrl-C), and the server acknowledges it with `{"status": "cancelled"}` -- but nothing actually happens. The engine loop continues running, consuming LLM API credits and tool executions. The user believes they cancelled but the system is still working.

This was noted in the system overview's open questions and the correctness review. From an operability perspective, the false acknowledgment is worse than returning an error -- it creates a trust gap between the UI and the system state.

**Recommendation:** Either implement cancellation via `CancellationToken` (the TODO comment already identifies the approach), or change the response to indicate that cancellation is not yet supported so the user knows to expect continued execution.
