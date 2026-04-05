---
id: service-layer-websocket-server
level: initiative
title: "Service Layer + WebSocket Server — headless Arawn daemon"
short_code: "ARAWN-I-0006"
created_at: 2026-04-01T10:37:12.669038+00:00
updated_at: 2026-04-02T12:35:44.034880+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: service-layer-websocket-server
---

# Service Layer + WebSocket Server — headless Arawn daemon Initiative

## Context

The TUI (I-0005) needs to talk to the engine. Rather than coupling them in-process and retrofitting a network layer later, we build the service abstraction and WebSocket server first. The TUI then starts life as a WebSocket client — no rework needed when we want multiple clients, remote access, or a daemon mode.

The engine, storage, tools, and compaction are all built and tested. This initiative wraps them in a clean service interface and exposes it over WebSocket.

### Reference
- ARAWN-I-0005: TUI design references ArawnService trait + EngineEvent stream
- Vision: "Future Directions — Headless Server Mode"
- All engine/storage/tool code from I-0001 through I-0004

## Goals & Non-Goals

**Goals:**
- `arawn-service` crate: `ArawnService` trait + `EngineEvent` + view types
- `LocalService` implementation: wraps engine + store + tools behind the trait
- WebSocket server: `arawn serve` command starts a daemon on localhost
- JSON protocol over WebSocket: request/response for CRUD, streaming for chat
- `arawn serve` runs headless — no terminal UI, just the server
- One-shot CLI (`arawn "prompt"`) still works (backward compatible)
- Integration tests: connect to WS server, send message, receive streamed events

**Non-Goals:**
- Authentication / TLS (localhost only in v1)
- Multi-user / multi-tenant
- Remote deployment (local daemon only)
- HTTP REST API (WebSocket only)
- The TUI itself (that's I-0005)

## Architecture

### Crate Dependency Graph

```
arawn-service (trait + types)
  ↑              ↑
  │              │
arawn-tui      arawn (binary)
  (future)       │
                 ├── local_service.rs  (ArawnService impl)
                 ├── ws_server.rs      (WebSocket handler)
                 │
                 ├── arawn-engine
                 ├── arawn-storage
                 ├── arawn-llm
                 └── arawn-tool-plugin
```

### arawn-service Crate

Tiny crate — just the trait and types:

```rust
#[async_trait]
pub trait ArawnService: Send + Sync {
    // Workstreams
    async fn list_workstreams(&self) -> Result<Vec<WorkstreamInfo>>;
    async fn create_workstream(&self, name: String, root_dir: PathBuf) -> Result<WorkstreamInfo>;

    // Sessions
    async fn list_sessions(&self, ws_id: Option<Uuid>) -> Result<Vec<SessionInfo>>;
    async fn create_session(&self, ws_id: Option<Uuid>) -> Result<SessionInfo>;
    async fn load_session(&self, id: Uuid) -> Result<SessionDetail>;

    // Chat
    async fn send_message(
        &self,
        session_id: Uuid,
        content: String,
    ) -> Result<Pin<Box<dyn Stream<Item = EngineEvent> + Send>>>;
    async fn cancel(&self, session_id: Uuid) -> Result<()>;
}

pub enum EngineEvent {
    StreamingText(String),
    ToolCallStart { id: String, name: String },
    ToolCallResult { id: String, content: String, is_error: bool },
    Complete { final_text: String },
    Error(String),
    CompactionOccurred { messages_summarized: usize },
}
```

Dependencies: `arawn-core`, `async-trait`, `futures`, `uuid`, `serde`.

### WebSocket Protocol

JSON messages over WebSocket. Request/response pairs identified by `id`. Streaming events have no `id` — they're pushed.

**Client → Server:**
```json
{"id": 1, "method": "list_workstreams"}
{"id": 2, "method": "create_session", "params": {"workstream_id": null}}
{"id": 3, "method": "send_message", "params": {"session_id": "abc", "content": "hello"}}
{"id": 4, "method": "cancel", "params": {"session_id": "abc"}}
```

**Server → Client (responses):**
```json
{"id": 1, "result": [{"id": "...", "name": "scratch"}]}
{"id": 2, "result": {"id": "...", "workstream_id": null}}
```

**Server → Client (streaming, no id):**
```json
{"event": "streaming_text", "data": {"text": "Hello"}}
{"event": "tool_call_start", "data": {"id": "c1", "name": "shell"}}
{"event": "tool_call_result", "data": {"id": "c1", "content": "...", "is_error": false}}
{"event": "complete", "data": {"final_text": "Here are the files..."}}
```

### LocalService

Refactors the existing `main.rs` wiring into a struct:

```rust
pub struct LocalService {
    store: Store,
    llm: Arc<dyn LlmClient>,
    registry: Arc<ToolRegistry>,
    model_limits: ModelLimits,
    system_prompt: String,
}
```

`send_message` spawns a tokio task that runs `QueryEngine::run` and bridges the session mutation into `EngineEvent` stream via a channel. The engine currently mutates `Session` directly — we need to intercept each message append and emit the corresponding `EngineEvent`.

### Binary Modes

```
arawn "prompt"        # One-shot CLI (existing, backward compat)
arawn serve           # Start WebSocket server on localhost:3100
arawn --list-sessions # Existing CLI flag
arawn --session <id>  # Existing CLI flag
```

## Detailed Design

### WebSocket Server

Use `axum` with WebSocket upgrade. Single endpoint: `ws://localhost:3100/ws`.

One WebSocket connection = one client session. The server holds an `Arc<LocalService>` shared across connections.

```rust
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(service): State<Arc<LocalService>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_connection(socket, service))
}
```

### Streaming Bridge

The current `QueryEngine::run` is synchronous from the caller's perspective — it returns the final text after all tool loops complete. For streaming, we need to emit events as they happen.

Options:
1. **Callback-based**: Add an `on_event` callback to the engine — invasive change
2. **Channel-based**: `LocalService::send_message` creates a channel, wraps Session in a proxy that emits events on `add_message`, runs engine in background task
3. **Refactor engine to return a stream** — cleanest long-term but biggest change

For v1: **option 2** (channel-based). The `LocalService` wraps the session with event emission, runs the engine in a spawned task, and returns the receiver as the `EngineEvent` stream. The engine itself doesn't change.

## Alternatives Considered

1. **gRPC instead of WebSocket** — Rejected. Adds protobuf/tonic complexity. WebSocket + JSON is simpler, debuggable with any WS client, and sufficient for single-user.
2. **HTTP SSE for streaming** — Rejected. WebSocket is bidirectional (needed for cancel). SSE is one-way.
3. **Build service layer later** — Rejected. Would mean building TUI in-process first and then ripping out the coupling. Do it right from the start.
4. **Skip the trait, just build the WS server** — Rejected. The trait enables testing the TUI without a server, and makes the local/remote distinction clean.

## Implementation Plan

Tasks to be decomposed after design approval:
1. arawn-service crate (trait + types)
2. LocalService implementation (refactor main.rs wiring)
3. WebSocket server with axum (serve command, JSON protocol)
4. Streaming bridge (engine → EngineEvent channel)
5. Wire into binary (arawn serve mode)
6. Integration tests (WS client → server → engine → response)