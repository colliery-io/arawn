---
id: backfill-migration-ceremony-state
level: task
title: "Backfill migration — ceremony state to todos"
short_code: "ARAWN-T-0311"
created_at: 2026-05-16T22:52:31.352253+00:00
updated_at: 2026-05-17T10:31:51.737379+00:00
parent: ARAWN-I-0049
blocked_by: [ARAWN-T-0309]
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

- [x] V8 inserts one `todos` row per `ceremony_priorities` row
      with `kind='weekly_priority'`, body from priorities.body,
      rationale from `NULLIF(priorities.rationale,'')`, `done_at`
      copied verbatim, `attrs` carrying `{tablet_id, ordinal,
      confirmed_at, citation_id}`. created_at sourced from the
      parent tablet's generated_at via JOIN.
- [x] V8 inserts one `todos` row per `ceremony_todos_rolling` row
      with `kind='rollover'`, body from rolling.body, `done_at`
      copied, `attrs` carrying `{origin_tablet_id,
      last_seen_tablet_id}`.
- [x] Source columns untouched — `ceremony_priorities` /
      `ceremony_todos_rolling` continue to be read by existing
      retro/daily/weekly plugins until [[ARAWN-T-0312]] swaps the
      reads.
- [x] Migration test: build a V7 db, seed ceremony state, apply
      V8, assert per-row equivalence (body, kind, done_at, attrs,
      created_at, rationale NULL coercion). Second test confirms
      V8 is a no-op on empty ceremony state.

## Status Updates

### 2026-05-17 — shipped

- Refinery V7 was already T-0309's `todos` table; the backfill
  ships as V8 (separate migration step). Append-only refinery
  conventions held.
- Deterministic IDs: `wp:<source_id>` for priorities,
  `rl:<source_id>` for rollover todos. Stable across re-runs
  against the same fixture; refinery's own version tracking
  handles real-database idempotence.
- `json_object()` builds `attrs` inline in SQL — no Rust-side
  serialization step needed.
- `NULLIF(rationale,'')` coerces empty-string rationale to NULL
  so consumers see the canonical "no rationale" state.
- Added `Database::in_memory_at_version` test helper that uses
  refinery's `Target::Version` API to seed pre-upgrade fixtures.
- 2 new storage unit tests; full storage suite 74/74 green.

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