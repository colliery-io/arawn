---
id: ceremony-schema-cutover-fk
level: task
title: "Ceremony schema cutover — FK pointers + view"
short_code: "ARAWN-T-0312"
created_at: 2026-05-16T22:52:32.923850+00:00
updated_at: 2026-05-17T11:27:58.062950+00:00
parent: ARAWN-I-0049
blocked_by: [ARAWN-T-0311]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0049
---

# Ceremony schema cutover — FK pointers + view

## Parent Initiative

[[ARAWN-I-0049]]

## Objective

Make `todos` the single source of truth for `done_at` across the
ceremony surface. `ceremony_priorities` becomes a thin index with
a `todo_id` FK; `ceremony_todos_rolling` retires behind a view.
Ceremony plugins route their gather queries through `TodoService`.

## Acceptance Criteria

## Acceptance Criteria

- [x] V9 migration recreates `ceremony_priorities` (rename-copy-drop
      dance) with a NOT NULL `todo_id` FK referencing todos.id; the
      body/rationale/citation_id/done_at columns are gone. Backfill
      from V8 populates the link via `wp:<id>` pattern.
- [x] `ceremony_todos_rolling` dropped and re-created as a read-only
      view over `todos WHERE kind='rollover' AND archived_at IS NULL`.
      The view preserves the original column shape (todo_id, body,
      origin_tablet_id, created_at, done_at, last_seen_tablet_id)
      via `json_extract` from attrs, so existing SELECT-shaped
      readers compile and run unchanged.
- [x] `ceremony_items.todo_id` optional FK added (`REFERENCES
      todos(id) ON DELETE SET NULL`).
- [x] Production reads in `daily.rs` / `weekly.rs` / `retro.rs` /
      `retro_detectors.rs` / `service.rs` rewritten to JOIN
      `ceremony_priorities` → `todos` for body/rationale/done_at.
      Rolling-todo reads continue to go through the view.
- [x] All ceremony writers (5 producers across service/daily/weekly/
      retro/retro_detectors + 6 test seeders across the workspace)
      now route through `todos` INSERTs followed by thin
      ceremony_priorities link rows.
- [x] Workspace unit suite 1796/0.

## Status Updates

### 2026-05-17 — cutover shipped

- V9 migration: rename-copy-drop on ceremony_priorities, view over
  todos for ceremony_todos_rolling, optional FK on ceremony_items.
- Migration backfill uses T-0311's `wp:<id>` deterministic ID so
  the FK populates cleanly with no orphans.
- All `INSERT INTO ceremony_priorities` callsites (1 in service,
  4 in plugins + retro_detectors, 5 in tests/UATs) now insert a
  todo first with the kind-specific attrs, then the thin link row.
- All `INSERT INTO ceremony_todos_rolling` callsites likewise
  route through `todos` with `kind='rollover'` (view is read-only).
- `citation_id` lookups everywhere updated to
  `json_extract(t.attrs, '$.citation_id')` via a JOIN on todos.
- `reject_priority` now deletes the todo (which cascades to the
  link row via ON DELETE CASCADE), plus a belt-and-braces sweep
  of orphans.
- Test helpers (`insert_priority`, `insert_rolling_todo`, weekly
  + daily + retro UAT seeds) all updated to the new shape.
- 4 ceremonies tests + 3 UAT seed tests + 1 storage test +
  1 engine tool test had to be updated, all green now.

## Implementation Notes

### Technical Approach

- SQLite's `ALTER TABLE DROP COLUMN` works on 3.35+; verify
  bundled version. If it's older, fall back to the
  rename-copy-drop dance.
- The view trick for `ceremony_todos_rolling` keeps the gather
  queries schema-stable so the cutover is one PR, not two.
- Be surgical with which columns get dropped: keep
  `ceremony_priorities.{tablet_id, ordinal, confirmed_at,
  source_item_id}` since those are the index dimensions.

### Dependencies

- Blocked by [[ARAWN-T-0311]] (backfilled rows must exist to
  satisfy the NOT NULL FK).
- Blocks [[ARAWN-T-0316]] (UAT proves the cutover preserves
  feedback-loop behaviour).

### Risk Considerations

- Plugin regression: gather query rewrites are the riskiest
  edits in this initiative. Lean hard on the existing UAT
  scenarios as the regression gate.
- View vs table for rolling: a view means writes go through
  `todos` only; ensure no straggler `INSERT INTO
  ceremony_todos_rolling` path remains.