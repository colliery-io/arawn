---
id: backfill-migration-ceremony-state
level: task
title: "Backfill migration — ceremony state to todos"
short_code: "ARAWN-T-0311"
created_at: 2026-05-16T22:52:31.352253+00:00
updated_at: 2026-05-16T22:52:31.352253+00:00
parent: ARAWN-I-0049
blocked_by: [ARAWN-T-0309]
effort: M
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0049
---

# Backfill migration — ceremony state to todos

## Parent Initiative

[[ARAWN-I-0049]]

## Objective

Migrate every existing `ceremony_priorities` and
`ceremony_todos_rolling` row into the new `todos` table without
dropping the source columns yet. Non-destructive — schema cutover
happens in [[ARAWN-T-0312]].

## Acceptance Criteria

- [ ] V7 (or a follow-up step of V7) inserts one `todos` row per
      `ceremony_priorities` row with `kind='weekly_priority'`,
      `body` from priorities.body, `rationale` from priorities.rationale
      (often empty pre-[[ARAWN-T-0315]]), `done_at` copied verbatim,
      `attrs` carrying `{tablet_id, ordinal, confirmed_at,
      citation_id}`.
- [ ] V7 inserts one `todos` row per `ceremony_todos_rolling` row
      with `kind='rollover'`, `body` from rolling.body, `done_at`
      copied, `attrs` carrying `{origin_tablet_id,
      last_seen_tablet_id}`.
- [ ] Source columns untouched — `ceremony_priorities` /
      `ceremony_todos_rolling` continue to be read by existing
      retro/daily/weekly plugins until [[ARAWN-T-0312]] swaps the
      reads.
- [ ] Migration test: build a V6 db with non-trivial ceremony
      state (use an existing UAT seed), run V7, assert row counts
      and per-row equivalence between source tables and the new
      todos rows.

## Implementation Notes

### Technical Approach

- Refinery migrations are append-only. If V7 has already been
  taken by [[ARAWN-T-0309]]'s `todos` table creation, this task
  adds V8 with the backfill inserts. Coordinate ordering during
  implementation.
- IDs: synthesise `todo_id` deterministically (e.g.
  `uuidv5(namespace, source_table || source_id)`) so re-running
  the migration on a re-created db gives stable IDs.

### Dependencies

- Blocked by [[ARAWN-T-0309]] (todos table must exist).
- Blocks [[ARAWN-T-0312]] (schema cutover needs backfilled rows
  to point FKs at).

### Risk Considerations

- Idempotence: a migration shouldn't double-insert on re-run.
  Refinery's version tracking handles this for the migration
  itself; deterministic IDs are belt-and-braces.

## Status Updates

*To be added during implementation*
