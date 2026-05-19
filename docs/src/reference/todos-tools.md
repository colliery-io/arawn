# Todos tools

*Reference. The eight `todo_*` tools the agent uses for the generic todo surface.*

Source: `crates/arawn-engine/src/tools/todo.rs`. Persistence: `arawn.db`. UI: the `/todo` modal in the TUI.

Todos are a generic surface (introduced in I-0049 / T-0314): the agent uses them for ceremony follow-ups, the user creates them directly via `/todo`, and they're queryable across workstreams.

## Tools

| Tool | Description |
|---|---|
| `todo_create` | Create a todo. Fields: title (required), body, workstream, source, due_at, tags. |
| `todo_list` | List todos. Optional filters: workstream, status, source, tags. |
| `todo_get` | Get one todo by short code. |
| `todo_done` | Mark a todo done. |
| `todo_undo` | Un-mark a done todo back to open. |
| `todo_patch` | Edit fields on an existing todo. |
| `todo_archive` | Archive (soft-delete). |
| `todo_search` | Free-text search across todo titles and bodies. |

## Schema

The `todos` table (source: `crates/arawn-storage` migrations V8 / V11):

| Field | Type | Notes |
|---|---|---|
| `short_code` | string | Stable id like `T-0042`. |
| `title` | string | Display title. |
| `body` | string | Optional longer description. |
| `workstream` | string | Owning workstream slug. Can be `scratch`. |
| `source` | string | Free-form origin tag — `ceremony/daily`, `ceremony/weekly/priority`, `user`, etc. |
| `status` | enum | `open` / `done` / `archived`. |
| `due_at` | RFC3339 | Optional due date. |
| `tags` | list&lt;string&gt; | Free-form tags. |
| `created_at` / `updated_at` / `done_at` / `archived_at` | RFC3339 | Lifecycle timestamps. |

## Sources

Todos can come from anywhere — that's the point of the generic surface:

| Source | Created by |
|---|---|
| `user` | `/todo` modal or direct `todo_create`. |
| `ceremony/daily` | `daily_add_todo` from inside a daily ceremony. |
| `ceremony/weekly/priority` | Backfilled from confirmed weekly priorities. |
| `ceremony/retro/<detector>` | Retro detector output. |
| `agent` | Agent-created via `todo_create` during a conversation. |

The source string is preserved through the lifecycle. Filtering by source (`todo_list { source: "ceremony/daily" }`) lets you see "what did the daily ceremony surface?" without scanning everything.

## Workstream binding

Todos belong to a workstream. The `/todo` modal shows todos for the active workstream by default; pass `--all` to see across workstreams. The agent tools default to the active workstream too, with an explicit `workstream` argument for cross-workstream queries.

## Migration history

V8 introduced the `todos` table. V11 (and the cutover in T-0312) collapsed the old ceremony-state-only todo table into the generic one; ceremony state now points at todo rows via `todo_short_code` FK plus a view for back-compat. See `crates/arawn-storage/migrations/V11__...` for the schema.

## Related

- [Ceremonies tools reference](./ceremonies-tools.md) — how daily/weekly/retro create todos.
- [Slash commands reference](./slash-commands.md) — `/todo`.
- [Agent tools reference](./agent-tools.md) — full tool catalog.
