---
id: t-g-split-arawn-tui-src-render-rs
level: task
title: "T-G: Split `arawn-tui/src/render.rs` — render by visual area"
short_code: "ARAWN-T-0403"
created_at: 2026-05-22T01:47:01.950424+00:00
updated_at: 2026-05-22T01:47:01.950424+00:00
parent: ARAWN-I-0054
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0054
---

# T-G: Split `arawn-tui/src/render.rs`

## Parent Initiative

[[ARAWN-I-0054]]

## Direction

23 free fns + tests in one 2,426-line file. Split into per-visual-area submodules under `render/`. The `render()` orchestrator stays in `mod.rs` and dispatches.

## Acceptance Criteria

- [x] `crates/arawn-tui/src/render.rs` deleted; replaced by `crates/arawn-tui/src/render/` directory.
- [x] One file per visual area.
- [x] `render()` orchestrator dispatches to submodule functions.
- [x] All workspace lib tests pass.

## Status Updates

### 2026-05-22 — landed

**Split executed via Python helper.** The 2,426-line file became 7 files under `render/`:

| File | Lines | Contents |
|---|---|---|
| `mod.rs` | 1,317 | `render()` orchestrator + `truncate_to` / `compact_tool_summary` / `truncate_for_display` helpers + `SPINNER_FRAMES` / `DASHBOARD_WIDTH` / `MIN_FOR_THREE_PANE` consts + inline `#[cfg(test)] mod tests` (1,113 test lines) |
| `chat.rs` | 482 | `render_chat` + `render_separator` + `render_empty_chat_brief` + `render_idle_hero` |
| `dashboard.rs` | 323 | `render_dashboard_pane` + `render_dashboard_brief` + `render_dashboard_actions` + 5 helpers (`push_action_rows`, `format_action_row`, `format_brief_date_line`, `format_calendar_row`, `detect_conflict`) |
| `input.rs` | 137 | `render_input` + `render_autocomplete` |
| `status_bar.rs` | 119 | `render_status_bar` + `format_tokens` |
| `sidebar.rs` | 74 | `render_sidebar` + `render_sidebar_tab` |
| `overlays.rs` | 50 | `render_toast_bar` + `render_oauth_heartbeat` |

**Cross-module visibility:**
- All submodule fns promoted to `pub(super)` so `mod.rs::render` can dispatch.
- Helpers (`truncate_to`, `compact_tool_summary`, `truncate_for_display`) → `pub(super)`.
- `SPINNER_FRAMES` const stayed `const` (accessed via `super::SPINNER_FRAMES`).
- `mod.rs` re-imports submodule fns via `use chat::*; use dashboard::*; ...` so the orchestrator can call them unqualified.

**Inline tests** (per `feedback_inline_tests`) stayed in `mod.rs`. They reference functions now in submodules — added explicit `use super::dashboard::{detect_conflict, format_brief_date_line, render_dashboard_actions, render_dashboard_brief}` to the test mod.

`cargo fix --lib -p arawn-tui` cleaned 36 over-broad import warnings across the submodules.

**Caveat:** `mod.rs` at 1,317 lines is over the 800-line target — but 1,113 of those are inline tests, leaving ~200 lines of operative code (orchestrator + small helpers).

**Validation:**
- `cargo check --workspace`: ✅ clean.
- `cargo build --workspace --release`: ✅ clean (35s).
- `cargo test --workspace --no-run`: ✅ clean.
- `cargo test --workspace --lib`: ✅ **1,758 tests pass**, 0 fail.
