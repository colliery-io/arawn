---
id: tui-interactive-terminal-interface
level: initiative
title: "TUI — Interactive terminal interface with ratatui"
short_code: "ARAWN-I-0005"
created_at: 2026-04-01T10:29:39.298506+00:00
updated_at: 2026-04-02T12:35:43.450914+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: tui-interactive-terminal-interface
---

# TUI — Interactive terminal interface with ratatui Initiative

## Context

Arawn is currently a one-shot CLI — run a command, get a response, exit. To be a daily-use tool, it needs an interactive terminal interface for chat, workstream navigation, and session management. The previous TUI attempt (ratatui-based) grew too large without testing and never stabilized. This time: headless testing from day one, clean separation of state/logic/rendering.

### Reference
- Vision doc: "TUI as primary interface (no web UI in v1)"
- Previous attempt: ratatui + crossterm, failed due to untested complexity
- Claude Code: React 19 + Ink for terminal UI (~140 components)

## Goals & Non-Goals

**Goals:**
- New `arawn-tui` crate, separate from the binary crate
- ratatui + crossterm for terminal rendering
- Headless `TestBackend` mode for automated rendering tests
- Three-panel layout: sidebar (workstreams/sessions), chat area, input bar
- Status bar showing current workstream + session
- Interactive chat loop: type → send → stream response → display
- Streaming display: show assistant text as it arrives, show tool calls inline
- Workstream switching: sidebar navigation, create new workstream
- Session management: list, create, resume, switch
- Focus model: Tab to cycle sidebar/chat/input
- Scroll: chat history scrollable, auto-scroll on new messages
- Keyboard shortcuts: Ctrl-C quit, Ctrl-N new session, Esc cancel

**Non-Goals:**
- Mouse support (keyboard-only in v1)
- Syntax highlighting for code blocks (plain text for now)
- Split panes / resizable panels
- Configuration UI (use config files)
- Action items / watcher display (separate initiative)

## Architecture

### Crate Structure

```
crates/arawn-service/           # Service trait + streaming types (tiny, no deps beyond arawn-core)
├── src/
│   ├── lib.rs              # ArawnService trait, EngineEvent, MessageStream
│   └── types.rs            # SessionInfo, WorkstreamInfo (lightweight view types)

crates/arawn-tui/               # TUI client — depends on arawn-service, NOT on engine/llm/storage
├── src/
│   ├── lib.rs              # Public API: run_tui(), TuiConfig
│   ├── app.rs              # App state machine (all mutable state)
│   ├── event.rs            # Event loop: terminal events + service events
│   ├── render.rs           # Pure rendering: App → Frame (testable)
│   ├── widgets/            # Custom ratatui widgets
│   │   ├── chat.rs         # Chat message list with streaming
│   │   ├── sidebar.rs      # Workstream/session navigation
│   │   ├── input.rs        # Text input with cursor
│   │   └── status.rs       # Status bar
│   ├── action.rs           # User actions (enum of all possible interactions)
│   ├── screenshot.rs       # Render to PNG via TestBackend
│   └── headless.rs         # TestBackend wrapper for headless testing

crates/arawn/                   # Binary — provides LocalService impl, wires everything
├── src/
│   ├── main.rs
│   └── local_service.rs    # LocalService: implements ArawnService with in-process engine+store
```

### Service Layer

`arawn-service` defines the contract between any UI client and the Arawn backend:

```rust
#[async_trait]
pub trait ArawnService: Send + Sync {
    // Workstreams
    async fn list_workstreams(&self) -> Result<Vec<WorkstreamInfo>>;
    async fn create_workstream(&self, name: &str, root_dir: &Path) -> Result<WorkstreamInfo>;

    // Sessions
    async fn list_sessions(&self, ws_id: Option<Uuid>) -> Result<Vec<SessionInfo>>;
    async fn create_session(&self, ws_id: Option<Uuid>) -> Result<SessionInfo>;
    async fn load_session(&self, id: Uuid) -> Result<SessionInfo>;

    // Chat (streaming)
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

The TUI depends only on this trait. The binary crate provides `LocalService` that wires `QueryEngine` + `Store` + `ToolRegistry` behind it. A future remote client would provide `RemoteService` that talks over HTTP/WebSocket.

### State / Logic / Rendering Separation

```
                    ┌──────────────┐
  Terminal Events → │  Event Loop  │ → Actions
  Engine Events  → │              │
                    └──────┬───────┘
                           │ Actions
                    ┌──────▼───────┐
                    │  App State   │  (mutable, owns all state)
                    │              │
                    └──────┬───────┘
                           │ &App (immutable borrow)
                    ┌──────▼───────┐
                    │  render()    │  (pure function, testable)
                    │              │
                    └──────┬───────┘
                           │ Frame
                    ┌──────▼───────┐
                    │   Backend    │  (CrosstermBackend or TestBackend)
                    └──────────────┘
```

**App** owns: current workstream, current session, message history, input buffer, focus state, scroll position, streaming state.

**render()** is a pure function: `fn render(app: &App, frame: &mut Frame)`. No side effects. This is the key to testability — tests create an `App` in a known state, call `render()` with `TestBackend`, and assert on the buffer.

**Event loop** runs two streams concurrently:
1. Terminal events (key presses, resize) via crossterm `EventStream`
2. Engine events (streaming text deltas, tool calls, completion) via a channel

### Engine Integration

The TUI talks to `ArawnService` (not the engine directly). When the user presses Enter:

1. TUI calls `service.send_message(session_id, content)` → gets a `Stream<EngineEvent>`
2. TUI event loop polls the stream alongside terminal events
3. Each `EngineEvent` updates `App` state → triggers re-render
4. On `Complete` → re-enable input, persist messages

The `LocalService` implementation (in the binary crate) runs the engine in a background tokio task and bridges `QueryEngine::run` to the `EngineEvent` stream. The TUI doesn't know or care whether the service is local or remote.

### Headless Testing

```rust
// In tests:
let mut app = App::new(/* mock store, mock engine channel */);
app.handle_action(Action::TypeChar('h'));
app.handle_action(Action::TypeChar('i'));
app.handle_action(Action::Submit);

let mut terminal = Terminal::new(TestBackend::new(80, 24))?;
terminal.draw(|f| render(&app, f))?;

let buffer = terminal.backend().buffer();
// Assert: input area is now empty (message sent)
// Assert: chat area contains "hi" as a user message
```

This tests the full render pipeline without a real terminal.

## UI/UX Design

### Layout

```
┌─────────────────────────────────────────────────────┐
│ Arawn  │ Workstream: scratch  │ Session: abc123     │ Status bar
├────────┴────────┬───────────────────────────────────┤
│                 │                                   │
│  Workstreams    │  Chat                             │
│  ─────────────  │                                   │
│  > scratch      │  You:                             │
│    Home Maint   │  What files are here?              │
│    Finances     │                                   │
│                 │  Arawn:                            │
│  Sessions       │  [shell: ls -la]                   │
│  ─────────────  │  Here are the files:               │
│  > current      │  - Cargo.toml                      │
│    2026-03-31   │  - crates/                         │
│    2026-03-30   │  ...                               │
│                 │                                   │
│                 │  Arawn: █ (streaming)               │
│                 │                                   │
├─────────────────┴───────────────────────────────────┤
│ > Type your message...                              │ Input bar
└─────────────────────────────────────────────────────┘
```

### Focus States

| Focus | Highlight | Keys |
|-------|-----------|------|
| Input | Input bar highlighted, cursor visible | Type, Enter=send, Esc=cancel |
| Sidebar | Sidebar highlighted | ↑↓=navigate, Enter=select, n=new |
| Chat | Chat area highlighted | ↑↓=scroll, PgUp/PgDn |

Tab cycles: Input → Sidebar → Chat → Input.

### Streaming Display

While the assistant is generating:
- Text appears character-by-character in the chat area
- A cursor/spinner shows generation is active
- Tool calls appear as `[tool: name]` blocks, expanding with results
- Input is disabled (grayed out) until generation completes

## Testing Strategy

### Headless Rendering Tests
- Every widget has tests that create App state → render → assert buffer contents
- Tests verify layout calculations, text wrapping, scroll behavior
- No real terminal needed — all use `TestBackend`

### State Machine Tests
- `App::handle_action` tested in isolation
- Verify: typing updates input buffer, submit sends message, focus cycling works
- Verify: streaming events update chat, completion re-enables input

### Integration Tests
- Full TUI loop with mock engine channel
- Send scripted events, verify final state

### Screenshot Mode
- `arawn --screenshot <path.png>` renders the current state to a PNG file
- Uses `TestBackend` to capture the buffer, then renders cells to an image with a monospace font
- Useful for documentation, bug reports, and visual regression testing
- Can be driven headlessly: set up state → screenshot → compare against reference image
- Library function: `screenshot(app, width, height, path)` — available for tests too

## Alternatives Considered

1. **Cursive** — Rejected. Less active than ratatui, smaller ecosystem.
2. **React/Ink (like Claude Code)** — Rejected. Requires Node.js runtime, violates "no heavy runtimes" constraint.
3. **TUI in the binary crate** — Rejected. Separate crate keeps the headless CLI usable and makes testing cleaner.
4. **Immediate-mode rendering without state separation** — Rejected. The previous attempt failed because state and rendering were tangled. Clean separation is the lesson learned.

## Implementation Plan

Tasks to be decomposed after design approval:
1. arawn-tui crate scaffolding + App state + headless TestBackend
2. Render pipeline: layout, status bar, empty chat, input bar
3. Input handling: typing, cursor, submit, focus cycling
4. Chat display: message rendering, scroll, tool call blocks
5. Streaming integration: engine channel, live text updates
6. Sidebar: workstream list, session list, navigation
7. Wire into binary: `arawn` command launches TUI by default, `arawn --headless` for one-shot mode
8. Headless rendering tests for all widgets