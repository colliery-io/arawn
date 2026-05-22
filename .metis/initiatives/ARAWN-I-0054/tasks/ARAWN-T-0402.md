---
id: t-f-split-arawn-tui-src-app-rs-app
level: task
title: "T-F: Split `arawn-tui/src/app.rs` — App state by concern"
short_code: "ARAWN-T-0402"
created_at: 2026-05-22T01:47:00.446848+00:00
updated_at: 2026-05-22T01:47:00.446848+00:00
parent: ARAWN-I-0054
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0054
---

# T-F: Split `arawn-tui/src/app.rs`

## Parent Initiative

[[ARAWN-I-0054]]

## Direction

`App` is the central TUI state. The `impl App` block was 895 lines holding 16 methods spanning many concerns (action dispatch, history recall, autocomplete, engine events, session load, conversation export). Split into per-concern inherent impl blocks under `app/`.

## Acceptance Criteria

- [x] `crates/arawn-tui/src/app.rs` deleted; replaced by `crates/arawn-tui/src/app/` directory.
- [x] One file per concern under `app/`.
- [x] Public API of `arawn_tui::app::*` unchanged.
- [x] `cargo check --workspace` clean.
- [x] `cargo build --workspace --release` clean.
- [x] `cargo test --workspace --no-run` clean.
- [x] All workspace lib tests pass.

## Status Updates

### 2026-05-22 — landed

**Split executed via Python helper.** The original 1,887-line `app.rs` became 5 files under `app/`:

| File | Lines | Contents |
|---|---|---|
| `mod.rs` | 1,078 | Type declarations (`LayoutRegions`, `Focus`, `SidebarSection`, `ChatMessage`, `ChatRole`, `App`, `HistoryEntry`, `DOUBLE_ESC_WINDOW`) + `App::new` + `post_toast` + `should_show_brief_in_empty_chat` + `prev/next_char_boundary` + module-level helpers (`default_export_path`, `shellexpand_tilde`, `render_conversation_markdown`, `format_tool_input`) + `Default for App` + inline `#[cfg(test)] mod tests` block |
| `actions.rs` | 534 | `handle_action` (412-line dispatch) + `handle_export_conversation` + `handle_copy_last_response` |
| `events.rs` | 158 | `apply_engine_event` + `load_session_messages` |
| `history.rs` | 121 | `record_input_history` + `history_recall_prev/next` + `open_history_modal` |
| `autocomplete.rs` | 48 | `update_autocomplete` + `accept_autocomplete` |

**Cross-module visibility:**
- Methods called from `actions.rs` but defined in `history.rs` / `autocomplete.rs` promoted to `pub(super)`.
- Free helpers (`default_export_path`, `shellexpand_tilde`, `render_conversation_markdown`, `format_tool_input`) promoted to `pub(super)`.
- `DOUBLE_ESC_WINDOW` stayed `pub const` (already was).
- All `App` fields stayed `pub` (already pub before split).

**Inline tests stayed in `mod.rs`** per `feedback_inline_tests`. The test block uses `use super::*; use crate::action::Action;` — `cargo fix` had removed the now-unused top-level `Action` import, so the test mod re-imports it explicitly.

**Caveat:** `mod.rs` at 1,078 lines is over the 800-line target, but ~600 of those are inline tests. The operative code is ~470 lines. Per `feedback_inline_tests`, tests stay inline.

**Validation:**
- `cargo check --workspace`: ✅ clean.
- `cargo build --workspace --release`: ✅ clean (35s).
- `cargo test --workspace --no-run`: ✅ clean.
- `cargo test --workspace --lib`: ✅ **1,758 tests pass**, 0 fail.
