---
id: ceremony-schema-cutover-fk
level: task
title: "Ceremony schema cutover — FK pointers + view"
short_code: "ARAWN-T-0312"
created_at: 2026-05-16T22:52:32.923850+00:00
updated_at: 2026-05-16T22:52:32.923850+00:00
parent: ARAWN-I-0049
blocked_by: [ARAWN-T-0311]
effort: M
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

- [ ] Migration adds `todo_id TEXT NOT NULL REFERENCES todos(id)
      ON DELETE CASCADE` to `ceremony_priorities` (populated from
      [[ARAWN-T-0311]]'s backfill); drops the duplicated columns
      `body`, `rationale`, `citation_id`, `done_at`.
- [ ] `ceremony_todos_rolling` table dropped (or renamed) and a
      view of the same name created over
      `SELECT ... FROM todos WHERE kind='rollover'`. Existing
      reads continue to work without code changes.
- [ ] `ceremony_items.kind="todo"` rows get an optional `todo_id`
      column (NULL for non-todo items, FK when set).
- [ ] `daily.rs` / `weekly.rs` / `retro.rs` gather queries route
      reads through `TodoService::list(...)` instead of raw
      table reads.
- [ ] Existing retro / daily / weekly unit tests + UATs pass
      unchanged.

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

## Status Updates

*To be added during implementation*
