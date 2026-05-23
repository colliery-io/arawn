---
id: agent-tool-family-todo-and
level: task
title: "Agent tool family — todo_* and ceremony tool migration"
short_code: "ARAWN-T-0313"
created_at: 2026-05-16T22:52:34.501946+00:00
updated_at: 2026-05-17T11:43:41.944504+00:00
parent: ARAWN-I-0049
blocked_by: [ARAWN-T-0310]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0049
---

# Agent tool family — todo_* and ceremony tool migration

## Parent Initiative

[[ARAWN-I-0049]]

## Objective

Give the agent a first-class todo surface usable anywhere in
chat, and migrate the ceremony todo/priority tools to thin
wrappers so there's a single mutation path.

## Acceptance Criteria

## Acceptance Criteria

- [x] New tools registered in `arawn-engine::tools::todo`:
      `todo_create`, `todo_list`, `todo_get`, `todo_done`,
      `todo_undo`, `todo_patch`, `todo_archive`, `todo_search`.
- [x] `todo_create` accepts `{body, workstream?, kind?, due_at?,
      rationale?}`. `kind` defaults to `"user"`; workstream
      defaults to NULL. RFC3339 due-date parsing built in.
- [x] Existing `daily_add_todo`, `weekly_confirm_priority`,
      `weekly_add_priority` already route through
      `CeremonyService` which post-T-0312 writes to the canonical
      `todos` table. No surface change required at the tool
      layer — the storage backplane swap is transparent.
- [x] `ToolCategory::Ceremony` keyword gate in query_engine.rs
      extended with `todo`, `reminder`, `remind me` triggers.
- [x] 8 per-tool unit tests under `tools::todo::tests` covering
      create round-trip, default-kind fallback, list filters,
      done/undo lifecycle, missing-id `todo_error`, patch, archive
      hide+restore, and search. Workspace 1804/0.

## Status Updates

### 2026-05-17 — shipped

- `crates/arawn-engine/src/tools/todo.rs` — 8 new tool structs.
  Each holds `Arc<Mutex<Store>>` + `Option<TodoEventSender>` and
  builds a `TodoService` on each execute() call. Events wired
  through `with_events(...)` so mutations propagate onto the
  notice broadcast.
- Tool registration unconditional in `main.rs` — not ceremony-
  gated, since todos are a chat-driven surface available even
  when ceremonies are off.
- Macro-based shared body for the single-id mutation tools
  (`todo_done`, `todo_undo`) keeps the file compact.
- `todo_get` returns `null` for missing rows (string literal,
  not the `Todo` shape), so it's inlined out of the macro.
- `todo_archive` returns `{status: "archived", id: ...}` rather
  than the row (since archive() returns ()).
- Existing ceremony wrapper tools (`daily_add_todo`,
  `weekly_confirm_priority`, `weekly_add_priority`) implicitly
  use the new schema — no edits needed, T-0312 did the heavy
  lifting at the storage layer.

### Risk Considerations

- Tool-surface stability: the agent's prompts reference current
  ceremony tool names. Wrappers must preserve names + arg shapes
  exactly to avoid prompt-engineering regressions in retro /
  daily / weekly flows.

### Dependencies

- Blocked by [[ARAWN-T-0310]] (RPC + events live).
- Blocks [[ARAWN-T-0314]] (TUI consumes these tools),
  [[ARAWN-T-0316]] (UAT uses `todo_done`).