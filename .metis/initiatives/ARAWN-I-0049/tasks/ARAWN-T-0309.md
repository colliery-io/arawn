---
id: todos-table-todoservice-scaffold
level: task
title: "todos table + TodoService scaffold"
short_code: "ARAWN-T-0309"
created_at: 2026-05-16T22:52:28.796013+00:00
updated_at: 2026-05-16T22:52:28.796013+00:00
parent: ARAWN-I-0049
blocked_by: []
effort: M
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0049
---

# todos table + TodoService scaffold

## Parent Initiative

[[ARAWN-I-0049]]

## Objective

Stand up the canonical `todos` table and a `TodoService` with full
CRUD. No callers yet — this is the substrate everything else in
I-0049 builds on.

## Acceptance Criteria

- [ ] Refinery V7 migration creates `todos` table per the
      [[ARAWN-I-0049]] sketch (id, body, rationale, kind,
      workstream, created_at, due_at, done_at, archived_at, attrs)
      with `kind`, `done_at`, `workstream` indexes.
- [ ] `TodoService` (new `arawn-storage::todos` module, no
      separate crate) exposes: `create / mark_done / undo /
      patch / archive / list(filter) / search(query) / get(id)`.
- [ ] `NewTodo` / `TodoPatch` / `ListFilter` types implement
      `serde::{Serialize, Deserialize}` for downstream RPC.
- [ ] `list` filter supports: kind, workstream, `done_at IS NULL`,
      due-window, free-text body LIKE for v1 (FTS deferred).
- [ ] Unit tests cover every service method including the
      idempotence of `mark_done` and the soft-delete semantics
      of `archive`.
- [ ] No production caller wired in yet — this task is scaffold-only.

## Implementation Notes

### Technical Approach

- Module path: `crates/arawn-storage/src/todos/{mod.rs,service.rs,types.rs}`.
- Reuse the existing `ConnHandle = Arc<Mutex<rusqlite::Connection>>`
  pattern; no separate connection pool.
- `attrs` is a `serde_json::Value` stored as TEXT — kind-specific
  payload (tablet_id, confirmed_at, ordinal, citation_id,
  origin_tablet_id, last_seen_tablet_id, ...). Don't index into it
  in v1; selection happens by `kind`.
- v1 search is plain `body LIKE '%q%'`. FTS5 deferred unless a
  follow-up needs it.

### Dependencies

- Refinery migration runner is already wired in `arawn-storage`.
- Foundation for [[ARAWN-T-0310]] (events + RPC), [[ARAWN-T-0311]]
  (backfill), [[ARAWN-T-0312]] (ceremony cutover),
  [[ARAWN-T-0313]] (agent tools).

### Risk Considerations

- Schema lock-in: get the column shape right now; later
  alterations on SQLite are painful. The sketch column list
  is intentionally minimal but extensible via `attrs`.

## Status Updates

*To be added during implementation*
