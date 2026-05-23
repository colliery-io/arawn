# API Design Review

## Summary

Arawn exposes seven API surfaces to its consumers: a WebSocket RPC protocol, the `ArawnService` trait, the `EngineEvent` streaming enum, the `Tool` trait, the `LlmClient` trait, a CLI interface, configuration via `arawn.toml`, and a plugin manifest format. The best-designed of these are the `LlmClient` trait (minimal, provider-agnostic, exactly one method) and the `EngineEvent` enum (well-tagged, complete, self-documenting). The weakest are the WebSocket RPC protocol (inconsistent naming, no schema, no versioning, ad-hoc error codes) and the `ArawnService` trait (covers only half the actual operations, creating a false abstraction boundary).

The most consequential design issue is the split between the `ArawnService` trait and the methods that bypass it. A client implementing against `ArawnService` gets session and message operations but misses memory, permissions, workflows, commands, and inventory -- over half the functional surface. This makes the trait unsuitable as an abstraction boundary and misleading as documentation of the system's capabilities.

## Interface Inventory

| Surface | Location | Consumers | Stability |
|---------|----------|-----------|-----------|
| WebSocket RPC | `ws_server.rs` | TUI, CLI, external clients | Unstable (no versioning) |
| `ArawnService` trait | `arawn-service/src/lib.rs` | `LocalService` (only impl) | Unstable |
| `EngineEvent` enum | `arawn-service/src/types.rs` | TUI, CLI, WS clients | Stable shape |
| `Tool` trait | `arawn-engine/src/tool.rs` | 22 built-in tools, plugins, MCP | Stable |
| `LlmClient` trait | `arawn-llm/src/client.rs` | 3 providers + mock + retry | Stable |
| CLI | `main.rs` | End users | Unstable (hand-rolled) |
| Configuration | `config.rs` / `arawn.toml` | Operators | Semi-stable |
| Plugin manifest | `plugins/manifest.rs` | Plugin authors | Semi-stable |

## Consistency Assessment

### Naming Conventions

The RPC method names are inconsistent across several dimensions:

1. **Verb patterns**: Some methods use bare nouns (`cancel`, `remember_fact`), some use verb_noun (`list_sessions`, `create_session`, `load_session`, `send_message`), and some use noun_noun (`memory_summary`). The overview doc says the methods are `remember`, `forget`, `memory_summary` but the actual RPC names are `remember_fact`, `forget_entity`, `memory_summary` -- mixing specificity levels.

2. **Noun patterns**: Sessions use `list_sessions` / `create_session` but the corresponding workstream operations are `list_workstreams` / `create_workstream`. Consistent, good. But then workflows use `list_workflows`, commands use `list_available_commands` (why "available"?), and permissions use `get_permission_mode` / `set_permission_mode` (switching to get/set verb style).

3. **Parameter naming**: Most ID parameters use `session_id` or `workstream_id` but `remember_fact` takes `text`, while `forget_entity` takes `query`, and `promote_session` takes `workstream_name` (a name, not an ID -- unlike every other workstream reference).

### Error Format

The RPC error format uses string `code` values, but these are ad-hoc and inconsistent:
- `"parse_error"` -- JSON parse failure
- `"service_error"` -- catch-all for any service-level error
- `"invalid_params"` -- missing required parameters
- `"method_not_found"` -- unknown method
- `"not_found"` -- no pending modal for the given request_id
- `"invalid_mode"` -- bad permission mode value

The `service_error` code covers everything from "session not found" to "storage I/O failure" to "engine crash," making it useless for programmatic error handling. The underlying `ServiceError` enum has `NotFound`, `InvalidOperation`, `Engine`, `Storage`, and `Internal` variants, but these are all flattened to `service_error` at the RPC layer.

### Return Format

Some methods return structured results (`list_sessions` returns `[{id, workstream_id, created_at}]`), while `query_inventory`, `remember_fact`, `memory_summary`, `forget_entity`, and `list_available_commands` all return opaque `serde_json::Value` directly from the service layer. The caller has no schema for what these return. The `promote_session` method also returns a raw `Value` rather than a typed response.

## Findings

---

### API-001: RPC Method Names Are Inconsistently Styled
**Severity**: Medium
**Location**: `crates/arawn/src/ws_server.rs:168-706`
**Confidence**: High

#### Description

The 16 RPC methods use four different naming patterns:
- **verb_noun**: `list_sessions`, `create_session`, `load_session`, `send_message`, `create_workstream`, `list_workstreams`, `list_workflows`, `promote_session`
- **verb_adjective_noun**: `list_available_commands` (why not `list_commands`?)
- **verb_noun (domain-specific)**: `remember_fact`, `forget_entity`, `query_inventory`
- **get/set_noun**: `get_permission_mode`, `set_permission_mode`
- **bare_verb**: `cancel`
- **noun_noun**: `memory_summary`
- **compound**: `user_input_response`

The overview document references these as `remember`, `forget`, `memory_summary`, `list_commands` -- but the actual RPC names are `remember_fact`, `forget_entity`, `list_available_commands`. The documentation and implementation have diverged.

#### Impact

External clients cannot predict method names. The `remember_fact` / `forget_entity` naming is semantically inconsistent with the overview's `remember` / `forget`. The `list_available_commands` name is unnecessarily verbose compared to the pattern established by `list_sessions` and `list_workstreams`.

#### Suggested Resolution

Standardize on `verb_noun` for all methods. Use consistent domain nouns: `list_commands` (not `list_available_commands`), `store_memory` / `delete_memory` / `get_memory_summary` (or `remember` / `forget` / `memory_summary` if brevity is preferred -- but pick one style). Use `get_` / `set_` consistently for state queries.

---

### API-002: `ArawnService` Trait Covers Half the Actual RPC Surface
**Severity**: Medium
**Location**: `crates/arawn-service/src/lib.rs`, `crates/arawn/src/ws_server.rs`
**Confidence**: High

#### Description

The `ArawnService` trait defines 7 methods: `list_workstreams`, `create_workstream`, `list_sessions`, `create_session`, `load_session`, `send_message`, `cancel`. The WebSocket server dispatches 16 methods. The remaining 9 (`promote_session`, `remember_fact`, `memory_summary`, `forget_entity`, `query_inventory`, `list_available_commands`, `list_workflows`, `get_permission_mode`, `set_permission_mode`, `user_input_response`) bypass the trait entirely and call `LocalService` directly.

The trait's doc comment says "Future: RemoteService" but a `RemoteService` built against this trait would be missing:
- Memory operations (remember, forget, summary)
- Permission management
- Workflow listing
- Session promotion
- Command/inventory queries
- User input response handling

#### Impact

The trait is a false abstraction. It suggests a clean service boundary exists, but over half the operations ignore it. A hypothetical remote service implementation would be incomplete. The trait gives new contributors an inaccurate picture of the system's contract.

#### Suggested Resolution

Either expand `ArawnService` to cover all RPC methods (the correct approach if `RemoteService` is still planned), or demote it to a narrower "session operations" trait and document that the full service API is the RPC protocol, not the trait.

---

### API-003: RPC Error Codes Discard Structured Error Information
**Severity**: Medium
**Location**: `crates/arawn/src/ws_server.rs` (all match arms), `crates/arawn-service/src/error.rs`
**Confidence**: High

#### Description

The `ServiceError` enum has five meaningful variants (`NotFound`, `InvalidOperation`, `Engine`, `Storage`, `Internal`), but the WebSocket handler maps all of them to the string `"service_error"`. The error message contains the `.to_string()` of the error, which is human-readable but not machine-parseable.

A client receiving `{"error": {"code": "service_error", "message": "not found: session abc123"}}` cannot programmatically distinguish "not found" from "storage failure" without parsing the message string.

Similarly, `LlmError` has well-structured variants with `is_retryable()` and `user_message()`, but these are flattened to a string by the time they reach the RPC layer.

#### Impact

Clients cannot implement proper error handling (e.g., retry on transient errors, show specific UI for "not found" vs "auth failure"). The structured error work in `LlmError` and `ServiceError` is wasted at the RPC boundary.

#### Suggested Resolution

Map `ServiceError` variants to distinct RPC error codes: `"not_found"`, `"invalid_operation"`, `"engine_error"`, `"storage_error"`, `"internal_error"`. Consider adding a `retryable` boolean to the error body for transient errors.

---

### API-004: No RPC Protocol Version or Capability Negotiation
**Severity**: Medium
**Location**: `crates/arawn/src/ws_server.rs`
**Confidence**: High

#### Description

The WebSocket RPC protocol has no version identifier. There is no handshake, no capability advertisement, and no way for a client to know which methods are available. A TUI built against a newer server (or vice versa) will silently fail when calling methods that don't exist or when receiving events it doesn't understand.

The `EngineEvent` enum uses `#[serde(tag = "event", content = "data")]` which is forward-compatible for adding new variants (unknown tags will fail to deserialize), but there is no mechanism for a client to gracefully handle new event types.

#### Impact

Any change to the RPC surface (adding methods, changing parameters, adding event variants) risks breaking existing clients. Since the server and TUI are separate binaries that may be updated independently (the server runs as a daemon), version mismatches are a realistic scenario.

#### Suggested Resolution

Add a `hello` or `handshake` method that the client calls on connection, returning the server version and supported method list. Alternatively, embed a version in the WebSocket URL path (e.g., `/ws/v1`). For events, clients should treat unknown event tags as no-ops rather than errors.

---

### API-005: `send_message` Streaming Protocol Mixes Framing Layers
**Severity**: Low
**Location**: `crates/arawn/src/ws_server.rs:323-457`
**Confidence**: High

#### Description

The `send_message` RPC is handled differently from all other methods. After parameter validation, it:
1. Sends a JSON-RPC `Response` with `{"status": "streaming"}` as the result
2. Switches to streaming raw `EngineEvent` JSON (not wrapped in a `Response` envelope)
3. Simultaneously accepts incoming `Request` messages (for `user_input_response`) on the same connection

This means the client must:
- Parse the first response as a standard JSON-RPC result
- Then parse subsequent messages as bare `EngineEvent` objects (different schema)
- Meanwhile, it can send `user_input_response` requests
- The stream end is signaled by the `Complete` or `Error` event variant, not by the JSON-RPC protocol

The events are sent as raw JSON without the `Response` wrapper (`id`, `result`, `error` fields). This breaks the JSON-RPC framing: a client parsing all messages as `Response` objects would fail to deserialize the events.

#### Impact

Any JSON-RPC client library will not work out of the box with this protocol. The client must implement a custom state machine that switches parsing modes after `send_message`. The inline `user_input_response` handling during streaming is undocumented and would surprise any client implementor.

#### Suggested Resolution

Consider wrapping streamed events in the `Response` envelope with a consistent `id` (the original request ID), e.g., `{"id": 42, "result": {"event": "StreamingText", "data": {"text": "..."}}}`. This keeps the protocol self-consistent. Alternatively, document the protocol state machine explicitly.

---

### API-006: Memory/Inventory Methods Return Untyped `serde_json::Value`
**Severity**: Low
**Location**: `crates/arawn/src/local_service.rs:133, 208, 226, 275, 316`
**Confidence**: High

#### Description

Five `LocalService` methods return `serde_json::Value` instead of typed structs:
- `query_inventory(kind: &str) -> Value`
- `list_available_commands() -> Value`
- `remember_fact(text: &str) -> Value`
- `memory_summary() -> Value`
- `forget_entity(query: &str) -> Value`

These methods construct ad-hoc JSON objects inline. There is no struct definition documenting the response shape, no serde derive for serialization consistency, and no way for the Rust type system to catch breaking changes.

For example, `remember_fact` returns either `{"status": "stored", "entities": [...]}` or `{"status": "error", "message": "..."}`. This is a return-type-as-error-handling pattern that bypasses the `Result` type and the `ServiceError` enum.

#### Impact

- Client code must use string-based field access (`result["status"].as_str()`)
- Response shapes can change silently without compile-time detection
- The methods bypass the error handling infrastructure (`ServiceError`, RPC error codes)
- A `RemoteService` implementation would need to reverse-engineer these JSON shapes

#### Suggested Resolution

Define typed response structs (e.g., `MemoryStoreResult`, `MemorySummary`, `CommandList`, `InventoryResult`). Return `Result<T, ServiceError>` instead of `Value`. Let the RPC layer handle serialization.

---

### API-007: `ToolContext` Is a God Object Passed to Every Tool
**Severity**: Low
**Location**: `crates/arawn-engine/src/context.rs`
**Confidence**: Medium

#### Description

Every tool receives a `ToolContext` reference containing: session ID, working directory, workstream name, allowed paths, an LLM client, model name, model limits, data directory, agent nesting depth, and a read-file tracker. Most tools use only 1-3 of these fields:
- `ThinkTool` uses none
- `GlobTool` / `GrepTool` use only `working_dir`
- `FileReadTool` uses `working_dir`, `allowed_paths`, `read_files`
- `AgentTool` uses almost everything

The context carries an `Arc<dyn LlmClient>` which gives every tool the capability to make arbitrary LLM API calls. The `ShellTool` does not need LLM access, but it receives it. This violates the principle of least privilege at the API level.

#### Impact

- New tool authors must understand the full `ToolContext` even if they use 10% of it
- The LLM client in the context is a capability leak -- tools could make untracked API calls
- Testing tools requires constructing a full `ToolContext` even for tools that barely use it (the `TestHarness` builder mitigates this but doesn't eliminate it)

#### Suggested Resolution

This is acceptable for the current scale (22 tools, single author). If the tool API is opened to third-party plugin authors, consider splitting into a `BasicToolContext` (working_dir, session_id, allowed_paths) and an `ExtendedToolContext` (adds LLM, model_limits, data_dir) that only capability-requiring tools receive.

---

### API-008: Plugin Manifest Uses camelCase but Config Uses snake_case
**Severity**: Low
**Location**: `crates/arawn-engine/src/plugins/manifest.rs`, `crates/arawn/src/config.rs`
**Confidence**: High

#### Description

The plugin manifest (`plugin.json`) uses `camelCase` field names (`mcpServers`, `userConfig`, `camelCase` via `#[serde(rename_all = "camelCase")]`). The system configuration (`arawn.toml`) uses `snake_case` field names (`network_tools`, `max_iterations`, `compaction_threshold`, `data_dir`). The RPC methods use `snake_case` (`list_sessions`, `create_workstream`). The `EngineEvent` uses `PascalCase` for variant tags (`StreamingText`, `ToolCallStart`).

This is three conventions across four interfaces:
- `camelCase`: plugin.json
- `snake_case`: arawn.toml, RPC methods, RPC parameters
- `PascalCase`: EngineEvent tags

#### Impact

A plugin author familiar with `arawn.toml` syntax will use the wrong casing in `plugin.json`. The `camelCase` choice for the manifest was made for Claude Code compatibility (a reasonable tradeoff), but the inconsistency is undocumented.

#### Suggested Resolution

Document the casing conventions explicitly. The current split is defensible: `camelCase` for JSON files matching Claude Code conventions, `snake_case` for TOML/RPC matching Rust conventions, `PascalCase` for enum variants matching serde's tagged enum convention. Just make the rationale visible.

---

### API-009: CLI Argument Parsing Is Hand-Rolled Without Validation
**Severity**: Low
**Location**: `crates/arawn/src/main.rs:29-89`
**Confidence**: High

#### Description

CLI arguments are parsed with a manual `while i < args.len()` loop. There is no structured help output (`--help` is not handled), no argument validation beyond UUID parsing, no subcommand disambiguation, and no shell completion support. Unknown arguments are silently collected into `prompt_parts`.

The `plugin` subcommand grabs `args[i+1..]` and passes the remainder, which works but means `arawn plugin --help` would need to be handled by the plugin subsystem specifically.

Error messages for malformed arguments are minimal:
- `--port` with no value silently defaults to 3100 (via `unwrap_or(3100)`)
- `--session` with an invalid UUID returns a proper error
- `--url` with no value silently does nothing (moves past `args.len()`)

#### Impact

Usability friction for new users. Missing `--help` is the most visible gap. Silent defaults on malformed `--port` values hide errors. No shell completion support limits discoverability.

#### Suggested Resolution

Adopt `clap` for argument parsing. This provides `--help`, `--version`, shell completions, and validation for free. The current argument surface is small enough that the migration would be straightforward.

---

### API-010: `promote_session` Takes a Workstream Name, Everything Else Takes IDs
**Severity**: Low
**Location**: `crates/arawn/src/ws_server.rs:533-568`
**Confidence**: High

#### Description

All workstream-related RPC methods use `workstream_id` (a UUID) to identify workstreams: `list_sessions`, `create_session`. But `promote_session` takes `workstream_name` (a string). This means the client must look up the workstream name externally, and the server must resolve the name to an ID internally (or create a new workstream).

This inconsistency means a client promoting a session must either:
1. Already know the workstream name (not the ID it used for `list_sessions`)
2. Call `list_workstreams` first to find names

#### Impact

Minor API friction. A client that has been working with workstream IDs throughout must switch to names for this one operation.

#### Suggested Resolution

Accept either `workstream_id` or `workstream_name` as parameters, with one taking precedence. Or accept only `workstream_id` for consistency, letting the client resolve names to IDs.

---

### API-011: `LlmClient` Trait Has Excellent Ergonomics (Positive Finding)
**Severity**: N/A (Positive)
**Location**: `crates/arawn-llm/src/client.rs`
**Confidence**: High

#### Description

The `LlmClient` trait is a single method: `stream(ChatRequest) -> Result<Stream<ChatChunk>, LlmError>`. The types are provider-neutral (`ChatRequest`, `ChatMessage`, `ChatContent`, `ToolDefinition`, `ChatChunk`). Adding a provider requires implementing one method. The `RetryClient` decorator wraps any `LlmClient` with exponential backoff using `is_retryable()`.

The `LlmError` enum is the strongest error type in the codebase: it classifies errors by HTTP status, provides `is_retryable()` for programmatic handling, and `user_message()` for display. It correctly distinguishes `Auth`, `ModelNotFound`, `RateLimited`, `ServerError`, `Stream`, `Config`, and `Api` variants.

This is the right abstraction at the right level: provider differences are absorbed into the implementation, and the consumer sees a clean streaming interface.

---

### API-012: `EngineEvent` Is Well-Designed but Missing Progress Information
**Severity**: Low
**Location**: `crates/arawn-service/src/types.rs:44-90`
**Confidence**: Medium

#### Description

The `EngineEvent` enum covers the streaming lifecycle well: `StreamingText`, `ToolCallStart`, `ToolCallResult`, `Complete`, `Error`, `CompactionOccurred`, `Usage`, `UserInputRequest`, `Flush`. The `serde(tag = "event", content = "data")` encoding is forward-compatible and self-describing.

However, there is no event for:
- **Iteration count / progress**: Clients cannot show "iteration 3 of 20" or estimate how far into the agentic loop the engine is
- **Tool call progress**: Long-running tool calls (shell commands, web fetch) have no intermediate progress indication beyond the initial `ToolCallStart`
- **Compaction progress**: `CompactionOccurred` fires after completion but there is no "compaction starting" event to explain a pause in output

The `Flush` event is an unusual inclusion -- it's a rendering hint rather than a semantic event. It couples the protocol to the TUI's rendering model.

#### Impact

The TUI shows tool names during execution but cannot show progress for long-running operations. The `Flush` event is harmless but is a leak of a client-specific concern into the protocol.

#### Suggested Resolution

Consider adding a `Progress { message: String }` event for long-running operations. The `Flush` event is fine to keep since it is cheap to ignore, but consider documenting it as a rendering optimization hint rather than a semantic event.
