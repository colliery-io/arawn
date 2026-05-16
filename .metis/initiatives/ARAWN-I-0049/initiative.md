---
id: generic-todo-system-ceremony-state
level: initiative
title: "Generic todo system + ceremony state refactor"
short_code: "ARAWN-I-0049"
created_at: 2026-05-16T22:22:56.668462+00:00
updated_at: 2026-05-16T22:59:52.947897+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: L
initiative_id: generic-todo-system-ceremony-state
---

# Generic todo system + ceremony state refactor

## Context

Three tables in `ceremony_*` carry partially-overlapping todo state today:

| Table                       | Purpose                              | `done_at` column? | Mutation path?                                |
| --------------------------- | ------------------------------------ | ----------------- | --------------------------------------------- |
| `ceremony_items` (kind=todo)| Items written under a tablet         | Yes               | `patch_item { done }`                         |
| `ceremony_priorities`        | Confirmed weekly priorities          | Yes               | None — `done_at` never gets set anywhere      |
| `ceremony_todos_rolling`     | Daily todos that carry across days   | Yes               | None — `done_at` never gets set anywhere      |

Retro's `priority_completion_ratio` detector reads
`ceremony_priorities.done_at` — and finds it always NULL, so the
detector either always fires (ratio = 0/N) or skips on N = 0.
`rollover_heat` reads `ceremony_todos_rolling.done_at` — same
problem. The retro feedback loop is structurally connected but
practically broken because nothing closes the "done" edge.

The shape of the missing mutation surface is identical for each
table — `mark_done(id) + undo(id) + WS event`. Building three of
them is duplicative; building one generic `todos` table that
ceremonies consume is the right architecture and unlocks a
broader "user-level todo list" feature available outside of
ceremonies (chat → "remind me to do X" → tool → done).

## Goals & Non-Goals

**Goals:**

- A single `todos` table with a `kind` discriminator covering
  user todos, ceremony rollover todos, and confirmed weekly
  priorities. All `done_at` mutations route through one service
  surface.
- Ceremony tables (`ceremony_priorities`, `ceremony_todos_rolling`)
  refactored to reference `todos.id` rather than duplicate
  body+state. `ceremony_items.kind="todo"` items likewise.
- Stand-alone agent tools (`todo_create`, `todo_list`,
  `todo_done`, `todo_undo`, `todo_search`) available outside
  ceremony contexts. The agent can manage a user's todo list in
  ordinary chat.
- A `/todo` TUI slash command parallel to `/today`/`/week`/`/retro`,
  rendering the user's open todos with done-toggle keybinds.
- Retro's `priority_completion_ratio` and `rollover_heat` detectors
  see real state transitions and fire only when warranted.
- Migration from existing rows is non-destructive: every existing
  ceremony_priorities + ceremony_todos_rolling row becomes a
  todos row with the right `kind`.

**Non-Goals:**

- External-system sync (Linear, GH issues, Jira). The schema
  should *permit* future kinds for these but this initiative
  doesn't ship them.
- Recurrence (every Tuesday at 10am). v1 todos are one-shot.
- Due-date reminders / notifications. The data model carries
  `due_at` but no reminder loop here.
- Deleting the existing ceremony tables — `ceremony_priorities`
  + `ceremony_todos_rolling` survive as thin indexes pointing
  into `todos` so retro/daily/weekly gather queries stay
  schema-stable.

## Design Decisions (locked 2026-05-16)

- **Single `todos` table with a free-form `kind` text discriminant.**
  `kind` is open-ended (not an enum) so future kinds — Linear sync,
  GH issues, etc. — drop in without schema changes.
- **Ceremony index tables become FK pointers.** `ceremony_priorities`
  keeps its row identity but body/state move to `todos`; `todo_id`
  is a foreign key. `ceremony_todos_rolling` retires behind a view
  over `todos WHERE kind='rollover'`.
- **User-todo workstream scoping**: defaults to `NULL` (global).
  Workstream tagging is explicit at create time (`todo_create
  { workstream: "..." }`), not auto-derived from conversation
  context.
- **No recurrence / reminders in v1.** Schema carries `due_at`
  for future use; nothing reads it on a schedule.
- **Rationale preservation: in scope for v1.** Weekly confirm flow
  copies the LLM-generated rationale into `todos.rationale` rather
  than discarding it.

## Detailed Design (sketch — to firm up in design phase)

### Data model

```sql
CREATE TABLE todos (
    id TEXT PRIMARY KEY,
    body TEXT NOT NULL,
    rationale TEXT,                       -- optional; preserved on weekly confirm
    kind TEXT NOT NULL,                   -- user | weekly_priority | rollover
    workstream TEXT,                      -- optional; ceremonies that span workstreams may leave null
    created_at TEXT NOT NULL,
    due_at TEXT,                          -- optional
    done_at TEXT,                         -- canonical done state
    archived_at TEXT,                     -- soft-delete on reject / pruning
    attrs TEXT NOT NULL DEFAULT '{}'      -- kind-specific JSON (tablet_id, confirmed_at, last_seen_tablet_id, ...)
);

CREATE INDEX todos_kind_idx        ON todos(kind);
CREATE INDEX todos_done_idx        ON todos(done_at);
CREATE INDEX todos_workstream_idx  ON todos(workstream);
```

Ceremony index tables become *pointers*:

```sql
-- ceremony_priorities now a thin link
DROP COLUMN body, rationale, citation_id, done_at;
ADD COLUMN todo_id TEXT NOT NULL REFERENCES todos(id) ON DELETE CASCADE;

-- ceremony_todos_rolling fully retires; replaced by:
--   SELECT * FROM todos WHERE kind = 'rollover' AND done_at IS NULL
-- Keep the table for one migration window (alias view), then drop in a follow-up.
```

`ceremony_items.kind = "todo"` items get an optional `todo_id`
column referencing the canonical row. The item is the "this
appeared in tablet X" record; the todo is the persistent state.

### Service surface

`crates/arawn-todos/` — new crate (or `arawn-storage::todos`
module if a new crate is overkill). `TodoService` with:

- `create(req: NewTodo) -> TodoDto`
- `mark_done(id) / undo(id) -> TodoDto`
- `patch(id, patch: TodoPatch) -> TodoDto` (body/rationale/due_at)
- `archive(id) -> ()` (soft-delete)
- `list(filter: ListFilter) -> Vec<TodoDto>` — kind, workstream, done_at IS NULL, due-window, fts
- `search(query: &str) -> Vec<TodoDto>`

Emits `TodoEvent::Created`, `::Completed`, `::Updated`,
`::Archived` on the existing notice broadcast.

### Migration

Refinery V7 migration:

1. Create `todos` table.
2. Insert one row per `ceremony_priorities` (kind="weekly_priority",
   attrs containing tablet_id+confirmed_at+ordinal+citation_id).
3. Insert one row per `ceremony_todos_rolling` (kind="rollover",
   attrs containing origin_tablet_id+last_seen_tablet_id).
4. Drop the duplicated columns on `ceremony_priorities`; add the
   `todo_id` link.
5. Retire `ceremony_todos_rolling` (rename or replace with a view
   over `todos`).

### Refactor zones

- **Ceremony plugins**: `daily.rs` + `weekly.rs` + `retro.rs`
  gather queries change from raw table reads to `todo_service.list(...)`.
- **Agent tools**: existing `daily_add_todo`, `weekly_confirm_priority`,
  `weekly_add_priority` become thin wrappers around `todo_service`
  calls. Brand-new `todo_*` family for cross-context use.
- **TUI**: existing modals call `todo_service.mark_done` instead
  of the deleted `patch_item { done }` path. New `/todo` command.

### Daily ↔ retro feedback loop closure

With the refactor, daily's todos section can offer a "mark done"
agent tool that hits `todo_service.mark_done(todo_id)`. Retro's
`rollover_heat` reads `todos WHERE kind="rollover" AND done_at IS
NULL AND created_at < monday AND last_seen IN this_week` and
sees genuine "still un-done" state. Priority-completion ratio
similarly reads real data.

## Alternatives Considered

- **Status quo (T-0309 band-aid)** — add `mark_done` mutation
  paths to each of the three existing tables independently.
  Quick win, closes the loop, but bakes duplication permanently.
  Rejected because the user-todo feature ask is real (the
  agent constantly says "I'll remember to ..." with no place
  to put it).
- **Replace ceremony_items + ceremony_priorities + ceremony_todos_rolling
  outright with a single tablet-agnostic todos table.** Cleaner
  end-state but the migration is brutal; retro/daily/weekly
  plugins need wholesale rewrites of their gather queries.
  Rejected for v1; this initiative keeps ceremony index tables
  as thin pointers so plugins change minimally.
- **External system as source of truth (Linear / GH issues)** —
  punted to a future initiative; the schema permits adding a
  new `kind` discriminator + sync adapter later without
  disturbing the local store.

## Implementation Plan

Decompose during design phase. Rough shape (probably 6-8 tasks):

1. `todos` table V7 migration + service crate scaffold (CRUD + tests).
2. `TodoEvent` broadcast variant + WS-RPC `todos.*` methods.
3. Backfill migration: ceremony_priorities + ceremony_todos_rolling
   → todos. Non-destructive.
4. Ceremony plugin refactor: gather queries route through
   TodoService. `ceremony_priorities` becomes a thin index;
   `ceremony_todos_rolling` retires behind a view.
5. Agent tool family: `todo_create / list / done / undo /
   search / patch / archive`. Migrate the daily + weekly tools
   to thin wrappers.
6. TUI `/todo` slash command + integration with existing
   `/today` `/week` modals' done-toggle keybinds.
7. UAT scenario: priority-completion feedback loop. Seed a
   weekly with confirmed priorities → run daily mid-week with
   `daily_mark_priority_done` (or `todo_done`) → run retro →
   judge sees `priority_completion_ratio` reflect the real
   ratio.
8. Rationale preservation — confirm flow copies the
   LLM-generated rationale into `todos.rationale` instead of `""`.

## Exit Criteria

- [ ] `todos` table is the single source of truth for done_at
      across user todos, weekly priorities, and rollover todos.
- [ ] Retro's `priority_completion_ratio` + `rollover_heat`
      detectors fire only when state actually warrants
      (verified in UAT).
- [ ] `/todo` slash command + `todo_*` agent tools work
      end-to-end outside any ceremony context.
- [ ] Existing retro / daily / weekly UATs continue to pass
      against the refactored gather paths.
- [ ] V7 migration backfills cleanly from a database produced
      by V6 with non-trivial ceremony state.
- [ ] Documentation: a short note on the `todos` shape in the
      ceremony engine docs explaining the kind discriminator.