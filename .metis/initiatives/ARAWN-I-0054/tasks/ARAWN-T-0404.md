---
id: t-h-split-arawn-tui-src-event-loop
level: task
title: "T-H: Split `arawn-tui/src/event_loop.rs` — event loop by event kind"
short_code: "ARAWN-T-0404"
created_at: 2026-05-22T01:47:03.059405+00:00
updated_at: 2026-05-22T01:47:03.059405+00:00
parent: ARAWN-I-0054
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0054
---

# T-H: Split `arawn-tui/src/event_loop.rs`

## Parent Initiative

[[ARAWN-I-0054]]

## Direction (decided at task start)

`event_loop.rs` was 2,280 lines: 33 top-level items dominated by `run_tui` (1,178-line `loop { select! { ... } }`). Like the `handle_connection` in T-D and the serve_mode block in T-E, the main loop is too coupled to extract without a context-struct rewrite. Scope-trimmed accordingly: extract helpers grouped by kind, leave `run_tui` intact in `mod.rs`.

## Acceptance Criteria

- [x] `crates/arawn-tui/src/event_loop.rs` deleted; replaced by `crates/arawn-tui/src/event_loop/` directory.
- [x] Helpers grouped by event kind (notices, ceremonies, brief, usage, todo) + formatting.
- [x] `run_tui` stays in `mod.rs`.
- [x] All workspace lib tests pass.

## Status Updates

### 2026-05-22 — landed (scope-trimmed)

**Split executed via Python helper.** Original 2,280-line file became 7 files under `event_loop/`:

| File | Lines | Contents |
|---|---|---|
| `mod.rs` | 1,400 | `run_tui` (1,178-line main loop) + `maybe_draw` / `force_draw` / `rect_contains` + inline tests |
| `ceremony.rs` | 408 | 13 fns: `current_iso_week`, `render_ceremony_today/week/retro`, `fetch_daily_view`, `fetch_weekly_view`, `fetch_tablet_id_and_status`, `fetch_diary_body`, `handle_ceremony_overlay_key`, `apply_priority_rpc_result`, `refresh_active_ceremony_overlay`, `fetch_items`, `fetch_priorities` |
| `formats.rs` | 257 | `format_integrations_list`, `OpenAttempt` enum, `try_open_url`, `format_permissions_status`, `format_feed_registered/list/discover`, `human_size`, `format_known_templates` |
| `todo.rs` | 119 | `fetch_open_todos`, `handle_todo_overlay_key` |
| `notices.rs` | 77 | `apply_system_notice`, `ceremony_event_should_refresh` |
| `brief.rs` | 44 | `refresh_brief_cache`, `render_brief_combined` |
| `usage.rs` | 33 | `render_usage` |

**Net reduction on the hotspot:** `event_loop.rs` was 2,280 lines; `mod.rs` is now 1,400 (-880, -39%). Doesn't hit the 800-line target — `run_tui` alone is 1,178 lines — but the helpers are out and the file's structure is cleaner.

**Cross-module wiring:**
- All extracted fns promoted to `pub(super)`.
- `mod.rs` re-imports submodule fns via `use brief::*; use ceremony::*; ...` so `run_tui` calls them unqualified (no source changes inside the giant loop).
- `brief.rs` cross-imports `current_iso_week`, `fetch_daily_view`, `fetch_weekly_view` from `super::ceremony` (the only cross-submodule dependency).

**Deferred:**
- Breaking up `run_tui` itself would require a `TuiContext` struct holding all the locals + a `dispatch_event(ctx, evt)` rewrite. Out of scope for I-0054.

**Validation:**
- `cargo check --workspace`: ✅ clean.
- `cargo build --workspace --release`: ✅ clean (42s).
- `cargo test --workspace --no-run`: ✅ clean.
- `cargo test --workspace --lib`: ✅ **1,758 tests pass**, 0 fail.
