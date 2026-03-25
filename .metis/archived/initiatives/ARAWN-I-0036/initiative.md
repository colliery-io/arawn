---
id: decompose-tui-app-architecture
level: initiative
title: "Decompose TUI App Architecture"
short_code: "ARAWN-I-0036"
created_at: 2026-03-22T23:50:09.908575+00:00
updated_at: 2026-03-25T02:00:36.469921+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: decompose-tui-app-architecture
---

# Decompose TUI App Architecture

## Context

`arawn-tui/src/app.rs` is a 3,272-line file with 95 public items in a single `App` struct. It handles chat state, sidebar state, input handling, WebSocket communication, session management, log viewing, focus management, and rendering coordination. This monolith makes it difficult to modify any single TUI feature without risk to others.

Promoted from blocked task ARAWN-T-0378.

## Goals & Non-Goals

**Goals:**
- Extract logical groups of methods into handler modules
- Reduce `App` to a coordinator that delegates to focused handlers
- Each handler should own its state and expose a clear interface
- Maintain identical TUI behavior — pure refactoring

**Non-Goals:**
- Adding new TUI features or panels
- Changing the rendering framework (ratatui)
- Adding TUI tests (separate initiative)

## Discovery Report: Full Method & Field Analysis

### File Stats

- File: `crates/arawn-tui/src/app.rs`
- Total lines: 3,272 (2,224 production, 1,048 test)
- Production `impl App` methods: 33
- Test-only `impl App` methods: 1 (`test_new`)
- Test helper functions: 4 (`key`, `key_mod`, `simulate_status_poll`, `test_app_controllable`)
- Test cases: 33
- Supporting types defined in-file: `PendingAction`, `InputMode`, `ChatMessage`, `ToolExecution`, `ContextState`, `UsageStats`, `DiskWarning`, `PanelAreas`

### App Struct Fields (38 total), Grouped by Functional Area

**WebSocket / Connection (4 fields)**
- `server_url: String`
- `ws_client: WsClient`
- `connection_status: ConnectionStatus`
- `last_ping: std::time::Instant`

**HTTP API (1 field)**
- `api: ArawnClient`

**Session State (5 fields)**
- `session_id: Option<String>`
- `workstream: String`
- `workstream_id: Option<String>`
- `reconnect_tokens: HashMap<String, String>`
- `is_session_owner: bool`

**Chat Display (4 fields)**
- `messages: BoundedVec<ChatMessage>`
- `waiting: bool`
- `chat_scroll: usize`
- `chat_auto_scroll: bool`

**Tool Pane (4 fields)**
- `tools: BoundedVec<ToolExecution>`
- `tool_scroll: usize`
- `selected_tool_index: Option<usize>`
- `show_tool_pane: bool`

**Input (3 fields)**
- `input: InputState`
- `input_mode: InputMode`
- `command_popup: CommandPopup`

**Sidebar (4 fields)**
- `sidebar: Sidebar`
- `moving_session_to_workstream: bool`
- `pending_delete_workstream: Option<(String, String)>`
- `pending_delete_session: Option<String>`

**Sessions Overlay (1 field)**
- `sessions: SessionList`

**Command Palette (1 field)**
- `palette: CommandPalette`

**Focus (1 field)**
- `focus: FocusManager`

**Log Panel (3 fields)**
- `log_buffer: LogBuffer`
- `log_scroll: usize`
- `show_logs: bool`

**Status / Chrome (3 fields)**
- `status_message: Option<String>`
- `context_name: Option<String>`
- `context_info: Option<ContextState>`

**Usage / Disk (3 fields)**
- `workstream_usage: Option<UsageStats>`
- `disk_warnings: Vec<DiskWarning>`
- `show_usage_popup: bool`

**Lifecycle (2 fields)**
- `should_quit: bool`
- `pending_actions: Vec<PendingAction>`

**Rendering (2 fields)**
- `panel_areas: PanelAreas`
- `command_executing: bool` / `command_progress: Option<String>`

---

### Method Inventory by Functional Area

#### 1. Chat Handling (5 methods)

| Method | Visibility | Fields Accessed | Cross-Cutting? |
|--------|-----------|----------------|---------------|
| `push_message` | private | `messages` | No |
| `push_tool` | private | `tools` | No |
| `send_message` | **pub** | `is_session_owner`, `status_message`, `input`, `messages`, `tools`, `chat_auto_scroll`, `ws_client`, `session_id`, `workstream_id`, `waiting` | Yes: input, ws_client, session state |
| `send_command` | private | `input`, `status_message`, `is_session_owner`, `ws_client`, `command_executing`, `session_id` | Yes: input, ws_client, session state |
| `get_help_text` | private | (none - pure) | No |

#### 2. Sidebar Handling (3 methods)

| Method | Visibility | Fields Accessed | Cross-Cutting? |
|--------|-----------|----------------|---------------|
| `handle_sidebar_key` | private | `sidebar`, `moving_session_to_workstream`, `pending_delete_*`, `focus`, `input_mode`, `input`, `status_message`, `session_id`, `pending_actions` | Yes: focus, input, session state |
| `handle_overlay_key` | private | `sidebar`, `focus` | Minor: focus |
| `clear_pending_deletes` | private | `pending_delete_workstream`, `pending_delete_session` | No |

#### 3. Input Handling (5 methods)

| Method | Visibility | Fields Accessed | Cross-Cutting? |
|--------|-----------|----------------|---------------|
| `handle_input_key` | private | `command_popup`, `input`, `input_mode`, `waiting`, `command_executing`, `status_message`, `sidebar`, `pending_actions`, `show_usage_popup`, `chat_scroll`, `chat_auto_scroll` | Yes: chat scroll, sidebar, pending actions |
| `update_command_popup` | private | `input`, `command_popup` | No |
| `scroll_chat_up` | private | `chat_auto_scroll`, `chat_scroll` | No |
| `scroll_chat_down` | private | `chat_auto_scroll`, `chat_scroll` | No |
| `build_command_args` | private | `session_id` (read-only) | Minor: session state |

#### 4. Log Handling (1 method)

| Method | Visibility | Fields Accessed | Cross-Cutting? |
|--------|-----------|----------------|---------------|
| `handle_logs_key` | private | `show_logs`, `focus`, `log_scroll`, `log_buffer` | Minor: focus |

#### 5. Session Management (4 methods)

| Method | Visibility | Fields Accessed | Cross-Cutting? |
|--------|-----------|----------------|---------------|
| `switch_to_session` | **pub** | `reconnect_tokens`, `ws_client`, `status_message`, `is_session_owner`, `messages`, `tools`, `session_id`, `sessions`, `sidebar`, `chat_scroll`, `chat_auto_scroll`, `pending_actions` | Yes: chat state, sidebar, ws_client |
| `create_new_session` | private | `messages`, `tools`, `session_id`, `chat_scroll`, `chat_auto_scroll`, `status_message` | Yes: chat state |
| `switch_to_workstream` | **pub** | `workstream`, `sidebar`, `workstream_id`, `messages`, `tools`, `session_id`, `chat_scroll`, `chat_auto_scroll`, `workstream_usage`, `pending_actions`, `status_message` | Yes: many areas |
| `open_sessions_panel` | private | `sessions`, `focus`, `sidebar` | Minor: focus, sidebar |

#### 6. Focus / Mouse (3 methods)

| Method | Visibility | Fields Accessed | Cross-Cutting? |
|--------|-----------|----------------|---------------|
| `handle_mouse` | private | `panel_areas`, `chat_scroll`, `chat_auto_scroll`, `tool_scroll`, `log_scroll` | Yes: scroll state for multiple panels |
| `panel_at` | private | `panel_areas` | No |
| `handle_key` | **pub** | `should_quit`, `waiting`, `ws_client`, `session_id`, `status_message`, `palette`, `focus`, `sidebar`, `show_tool_pane`, `selected_tool_index`, `tools`, `show_logs`, `show_usage_popup` | Yes: global dispatcher, touches everything |

#### 7. WebSocket / Server Messages (1 method)

| Method | Visibility | Fields Accessed | Cross-Cutting? |
|--------|-----------|----------------|---------------|
| `handle_server_message` | **pub** | `session_id`, `messages`, `waiting`, `tools`, `show_tool_pane`, `selected_tool_index`, `status_message`, `is_session_owner`, `command_executing`, `command_progress`, `context_info`, `disk_warnings`, `workstream_usage`, `workstream_id`, `reconnect_tokens` | Yes: touches nearly every field group |

#### 8. Async API Operations (7 methods)

| Method | Visibility | Fields Accessed | Cross-Cutting? |
|--------|-----------|----------------|---------------|
| `process_pending_actions` | private | `pending_actions` (dispatches to others) | Coordinator |
| `do_create_workstream` | private | `api`, `status_message`, `sidebar`, `workstream` | Yes: sidebar |
| `do_rename_workstream` | private | `api`, `status_message`, `sidebar`, `workstream` | Yes: sidebar |
| `do_delete_session` | private | `api`, `status_message`, `sidebar`, `session_id`, `messages`, `tools` | Yes: sidebar, chat |
| `do_delete_workstream` | private | `api`, `status_message`, `sidebar`, `workstream_id`, `workstream`, `messages`, `tools`, `session_id` | Yes: sidebar, chat, session |
| `do_fetch_workstream_sessions` | private | `api`, `sidebar`, `session_id` | Minor: session state |
| `do_fetch_session_messages` | private | `api`, `messages`, `chat_auto_scroll`, `status_message` | Yes: chat state |
| `do_move_session_to_workstream` | private | `api`, `sidebar`, `status_message`, `pending_actions` | Minor: sidebar |
| `refresh_sidebar_data` | private | `api`, `sidebar`, `workstream`, `workstream_id`, `status_message` | Yes: session state |

#### 9. Tool Pane (2 methods)

| Method | Visibility | Fields Accessed | Cross-Cutting? |
|--------|-----------|----------------|---------------|
| `handle_tool_pane_key` | private | `show_tool_pane`, `selected_tool_index`, `focus`, `tools`, `tool_scroll` | Minor: focus |
| `open_tool_in_editor` | private | `selected_tool_index`, `tools`, `status_message` | No |

#### 10. Palette / Actions (2 methods)

| Method | Visibility | Fields Accessed | Cross-Cutting? |
|--------|-----------|----------------|---------------|
| `handle_palette_key` | private | `palette`, `focus` | Minor: focus |
| `execute_action` | private | `focus`, `sidebar`, `status_message`, `session_id`, `moving_session_to_workstream`, `input_mode`, `input`, `should_quit`, `show_tool_pane` | Yes: orchestrates many areas |

#### 11. Sessions Overlay (1 method)

| Method | Visibility | Fields Accessed | Cross-Cutting? |
|--------|-----------|----------------|---------------|
| `handle_sessions_key` | private | `sessions`, `focus`, `session_id` | Minor: focus, session |

#### 12. Main Loop (2 methods)

| Method | Visibility | Fields Accessed | Cross-Cutting? |
|--------|-----------|----------------|---------------|
| `new` | **pub** | All fields (constructor) | N/A |
| `run` | **pub** | `should_quit`, `ws_client`, `connection_status`, `waiting`, `status_message`, `last_ping`, `pending_actions` | Coordinator |

#### 13. External Editor (1 method)

| Method | Visibility | Fields Accessed | Cross-Cutting? |
|--------|-----------|----------------|---------------|
| `run_pager` | private | (none - `&self` only for borrow) | No |

---

### High Cross-Cutting Methods (the tricky ones)

These methods touch 3+ field groups and will need careful refactoring:

1. **`handle_server_message`** - Touches nearly every field group. Must be split into sub-handlers that the coordinator dispatches to (e.g., `chat_handler.handle_chunk()`, `tool_handler.handle_tool_start()`).

2. **`handle_key`** (global shortcuts) - Coordinator by nature. Should remain on `App` as it dispatches to sub-handlers. Already partially delegated via `match self.focus.current()`.

3. **`switch_to_session`** - Resets chat state, updates sidebar, sends WebSocket subscribe, queues pending actions. Needs to call into multiple handlers.

4. **`switch_to_workstream`** - Similar to `switch_to_session` but broader. Resets session, chat, sidebar, usage state.

5. **`handle_sidebar_key`** - Deep coupling between sidebar navigation and session/workstream switching, input mode changes, and pending actions.

6. **`handle_input_key`** - Interleaves input editing with chat scroll, command dispatch, and workstream creation validation.

7. **`execute_action`** - Palette action dispatcher. Coordinator by nature, should stay on `App`.

8. **`send_message`** / **`send_command`** - Bridge input, WebSocket, and chat state.

---

### Proposed Handler Module Boundaries

#### Module 1: `handlers/log.rs` — `LogHandler`
**Fields:** `log_buffer`, `log_scroll`, `show_logs`
**Methods:** `handle_logs_key`
**Independence:** HIGH. Only cross-cutting field is `focus` (passed as `&mut FocusManager` param).
**Lines moved:** ~35

#### Module 2: `handlers/tool_pane.rs` — `ToolPaneHandler`
**Fields:** `tools`, `tool_scroll`, `selected_tool_index`, `show_tool_pane`
**Methods:** `handle_tool_pane_key`, `open_tool_in_editor`, `run_pager`, `push_tool`
**Independence:** HIGH. Focus is the only external dependency (pass as param).
**Lines moved:** ~120

#### Module 3: `handlers/palette.rs` — (already exists as `palette.rs`)
**Fields:** `palette` (already in `CommandPalette`)
**Methods:** `handle_palette_key` can move; `execute_action` must stay on `App` (coordinator).
**Independence:** HIGH.
**Lines moved:** ~40

#### Module 4: `handlers/sessions.rs` — `SessionsOverlayHandler`
**Fields:** `sessions` (already in `SessionList`)
**Methods:** `handle_sessions_key`, `open_sessions_panel`
**Independence:** MEDIUM. Calls `switch_to_session` and `create_new_session` on App.
**Lines moved:** ~50

#### Module 5: `handlers/chat.rs` — `ChatState`
**Fields:** `messages`, `waiting`, `chat_scroll`, `chat_auto_scroll`
**Methods:** `push_message`, `scroll_chat_up`, `scroll_chat_down`
**Independence:** MEDIUM. `send_message` bridges chat + input + ws_client so stays on App.
**Lines moved:** ~30 (state + scroll methods; send_message stays on App)

#### Module 6: `handlers/sidebar.rs` — `SidebarKeyHandler` (extends existing `sidebar.rs`)
**Fields:** `sidebar` (already in `Sidebar`), `moving_session_to_workstream`, `pending_delete_workstream`, `pending_delete_session`
**Methods:** `handle_sidebar_key`, `handle_overlay_key`, `clear_pending_deletes`
**Independence:** LOW. Heavy coupling to session switching, input mode, pending actions.
**Lines moved:** ~250

#### Module 7: `handlers/server_msg.rs` — Server message dispatch
**Methods:** `handle_server_message` (split into sub-match arms that call individual handlers)
**Independence:** LOW. Coordinator method. Best approach: keep the match on `App`, but have each arm call `self.chat.handle_chunk()`, `self.tools.handle_tool_start()`, etc.
**Lines moved:** ~230 (the match stays, but logic moves into handlers)

#### Module 8: `handlers/api_ops.rs` — Async API operations
**Fields:** `api`, `pending_actions`
**Methods:** `process_pending_actions`, `do_create_workstream`, `do_rename_workstream`, `do_delete_session`, `do_delete_workstream`, `do_fetch_workstream_sessions`, `do_fetch_session_messages`, `do_move_session_to_workstream`, `refresh_sidebar_data`
**Independence:** LOW. Every method reads/writes sidebar, session, and chat state.
**Lines moved:** ~300

---

### Recommended Extraction Order (most independent first)

| Order | Module | Risk | Rationale |
|-------|--------|------|-----------|
| 1 | `handlers/log.rs` | Trivial | 3 fields, 1 method, zero cross-cutting beyond focus param |
| 2 | `handlers/tool_pane.rs` | Low | 4 fields, 4 methods, only focus is external. `run_pager` is fully self-contained |
| 3 | `handlers/palette.rs` (extend existing) | Low | `handle_palette_key` is self-contained; `execute_action` stays on App |
| 4 | `handlers/sessions_overlay.rs` | Low | 1 method + open helper, callbacks go through App |
| 5 | `handlers/chat.rs` | Medium | State extraction is clean; `send_message`/`send_command` stay on App initially |
| 6 | `handlers/sidebar.rs` (extend existing) | Medium | Large method with many branches; move delete-confirm fields too |
| 7 | `handlers/server_msg.rs` | Medium | Split match arms to dispatch to typed handlers |
| 8 | `handlers/api_ops.rs` | High | Every method cross-cuts; extract last once other handlers own their state |

### Key Design Decisions Needed

1. **Handler trait vs plain structs?** Plain structs with methods are simpler. A trait would add indirection without clear benefit since there is only one implementation.

2. **How do handlers access shared state?** Two options:
   - (a) Handlers are fields on `App`; methods take `&mut self` params for cross-cutting state (e.g., `fn handle_logs_key(&mut self, focus: &mut FocusManager)`)
   - (b) `App` destructures into handler borrows at call sites
   - Recommendation: Option (a) is simpler and Rust borrow-checker friendly since each handler owns disjoint fields.

3. **Where do `ChatMessage`, `ToolExecution`, etc. live?** Move to a `types.rs` in the TUI crate or into their respective handler modules. `PendingAction` and `InputMode` stay on App since they cross module boundaries.

4. **FocusManager already extracted.** It exists at `crates/arawn-tui/src/focus.rs`. No work needed there.

5. **Test organization.** Tests currently construct `App::test_new()` and test integrated behavior. During extraction, keep integration tests in `app.rs`; add unit tests in each handler module for handler-specific logic.