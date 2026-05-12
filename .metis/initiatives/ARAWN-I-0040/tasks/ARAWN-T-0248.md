---
id: workstream-registry-lazy-kb
level: task
title: "Workstream registry + lazy KB binding + scratch default"
short_code: "ARAWN-T-0248"
created_at: 2026-05-12T23:25:49.418998+00:00
updated_at: 2026-05-12T23:25:49.418998+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Workstream registry + lazy KB binding + scratch default

## Parent Initiative

[[ARAWN-I-0040]]

## Objective

Stand up the workstream registry — the primary scope abstraction the rest of Phase 3+ hangs off of. A workstream is "a thing you track" (a person, a project, a hobby, an initiative). User-side: ~15 active expected, 50 is the high-side. Per-workstream KB lives in its own sqlite file under `~/.arawn/workstreams/<name>/memory.db`, opened lazily on first read/write.

The `scratch` workstream is auto-created on first boot, can't be deleted, and is the perpetual catch-all when a session hasn't picked a real workstream.

T-0249 layers slash commands on top of this; T-0250 wires sessions + memory routing to the active workstream.

## Scope

### Registry table

Add a `workstreams` table to `arawn.db` (the existing sqlite file `arawn-storage` already owns):

```sql
CREATE TABLE IF NOT EXISTS workstreams (
    name           TEXT PRIMARY KEY,          -- short, machine-safe (slug)
    display_name   TEXT NOT NULL,             -- human label
    description    TEXT NOT NULL DEFAULT '',  -- feeds extractor prompts in Phase 4
    bindings       TEXT NOT NULL DEFAULT '[]',-- JSON array of feed_ids
    archived       INTEGER NOT NULL DEFAULT 0,
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL
);
```

`name` is the addressing primitive (used in slash commands, in the file path). Reserved name: `scratch`. Validation: `^[a-z0-9][a-z0-9_-]*$`, 1-64 chars.

### `WorkstreamRegistry` (Rust)

Lives in `arawn-core` (next to existing `Workstream` shell type) — let me think... actually probably its own crate `arawn-workstreams` if it's going to grow. Decide as part of implementation; the existing `arawn-core::Workstream` is minimal and should be either absorbed or extended.

API surface (synchronous; takes `&rusqlite::Connection`):

- `WorkstreamRegistry::ensure_scratch(conn)` — idempotent; creates the `scratch` row if absent. Called on store boot.
- `new(conn, name, display_name, description) -> Result<Workstream, WorkstreamError>` — validates name, refuses `scratch`, errors on duplicate.
- `get(conn, name) -> Result<Option<Workstream>>`.
- `list(conn) -> Result<Vec<Workstream>>` (skips archived by default; flag to include).
- `update_description(conn, name, description)`.
- `set_bindings(conn, name, &[feed_id])` / `add_binding(conn, name, feed_id)` / `remove_binding(conn, name, feed_id)`.
- `delete(conn, name)` — refuses for `scratch`; soft-delete (set `archived = 1`) rather than hard delete so the KB file isn't orphaned silently. Hard-delete is a separate explicit op.

### Lazy KB binding

Extend `arawn-memory::MemoryManager` to accept a workstream name:

```rust
MemoryManager::for_workstream(global: Arc<MemoryStore>, name: &str, root: &Path) -> MemoryManager
```

The workstream KB at `<root>/workstreams/<name>/memory.db` is opened on first access. Memory manager caches the open store keyed by name. The `<global>` store stays as-is (one file at `~/.arawn/memory/global.db` — already exists today).

Lazy is preferred over eager — at 15-50 workstreams, eager boot wouldn't be painful, but lazy keeps cold-start cheap and avoids loading a vec0 + FTS5 index for a KB you may not query this session.

### Bindings are metadata only

The `bindings` column stores `[feed_id, …]` as JSON. Phase 3 doesn't act on bindings — the extractor in Phase 4 reads them to scope per-workstream extraction. T-0248 ships them as stored-but-inert. Same for `description`.

### What's deferred

- Slash commands — T-0249.
- Session-workstream binding + memory tool routing — T-0250.
- Per-workstream extraction (Phase 4) — bindings get acted on then.
- Workstream KB deletion / archival cleanup — soft-delete sets `archived`, file stays.
- "Promotion" (move entities scratch → real workstream) — explicit follow-up; logged in I-0040.

## Acceptance Criteria

- [ ] `workstreams` table created via `arawn-storage` migration; `scratch` row inserted on first boot.
- [ ] `WorkstreamRegistry` CRUD round-trips through the table.
- [ ] Name validation refuses bad slugs (`Pat!`, `..`, empty, > 64 chars) and reserves `scratch`.
- [ ] `MemoryManager::for_workstream(name)` returns a memory manager whose `workstream` store points at `~/.arawn/workstreams/<name>/memory.db`; the file is created on first write, not on construction.
- [ ] Delete is soft (sets `archived = 1`); `scratch` cannot be deleted.
- [ ] Unit tests cover: creation, validation, get/list, bindings round-trip, soft-delete, scratch protection.
- [ ] `angreal check workspace` + `angreal check clippy` clean.

## Implementation Notes

### Crate placement

`WorkstreamRegistry` lives where it has access to both `arawn-storage` (for the connection) and `arawn-core::Workstream` (or replaces it). My lean: extend `arawn-core` rather than a new crate — the type is small and avoiding crate proliferation matters.

### Migration

`arawn-storage` already has a migrations system (refinery). Add a new migration file `crates/arawn-storage/migrations/<version>__workstreams.sql`. The `scratch` row insertion can either be in the migration (idempotent INSERT OR IGNORE) or in `WorkstreamRegistry::ensure_scratch` called on boot. Latter is more flexible — pick that.

### Dependencies

- `arawn-storage` (migration + connection).
- `arawn-core` (Workstream type extension).
- `arawn-memory::MemoryManager` (`for_workstream` constructor).

### Risk considerations

- **`Workstream` type drift.** There's already an `arawn-core::Workstream`. Make sure the new registry doesn't fork the type — either absorb the existing one or replace it. Surfacing in the implementation.
- **KB path collision.** Two workstreams whose names slug-collide would share a KB. The slug validation regex prevents most cases; `name TEXT PRIMARY KEY` handles the rest at the DB level.
- **Archived workstreams.** Soft-deleted workstreams keep their KB on disk. Document the manual cleanup path so we don't accumulate orphaned files invisibly.

## Status Updates

*To be added during implementation*