---
id: arawn-storage-crate-scaffolding
level: task
title: "arawn-storage crate scaffolding + SQLite + refinery migrations"
short_code: "ARAWN-T-0009"
created_at: 2026-03-31T22:49:34.824056+00:00
updated_at: 2026-03-31T23:06:12.501397+00:00
parent: ARAWN-I-0002
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0002
---

# arawn-storage crate scaffolding + SQLite + refinery migrations

## Parent Initiative
[[ARAWN-I-0002]]

## Objective
Create the `arawn-storage` crate with SQLite connectivity via `rusqlite`, schema migrations via `refinery`, and the initial V1 migration. This is the foundation all persistence builds on.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `crates/arawn-storage/` added to workspace with `Cargo.toml`
- [ ] Dependencies: `rusqlite` (with `bundled` feature), `refinery` (with `rusqlite` backend), `arawn-core`, `tokio`, `thiserror`, `uuid`, `chrono`
- [ ] `crates/arawn-storage/migrations/V1__initial.sql` with workstreams + sessions tables
- [ ] `Database` struct that opens/creates an SQLite file and runs refinery migrations on construction
- [ ] `Database::open(path)` — opens existing or creates new DB, runs pending migrations
- [ ] `Database::in_memory()` — for testing
- [ ] Migrations run automatically and idempotently (running twice is safe)
- [ ] Declarative filesystem layout schema — a `DataLayout` struct that declares the expected directory tree (versioned, so it can evolve)
- [ ] `DataLayout::ensure(data_dir)` — reconciles actual directories against the declaration, creates what's missing
- [ ] Layout V1 declares: `workstreams/`, `scratch/sessions/`
- [ ] Test: open in-memory DB, verify tables exist via raw SQL query
- [ ] Test: run migrations twice, no error
- [ ] Test: DataLayout::ensure creates expected directories on fresh dir
- [ ] Test: DataLayout::ensure is idempotent (run twice, no error)
- [ ] `StorageError` error type in `error.rs`
- [ ] Workspace compiles, all existing tests still pass

## Implementation Notes
- `lib.rs`, `database.rs`, `layout.rs`, `error.rs`, `migrations/` in `crates/arawn-storage/src/`
- refinery embeds migrations at compile time via `embed_migrations!` macro
- Use `rusqlite::Connection` directly (not pooled) — single-user app, one connection is fine
- V1 migration SQL:
  ```sql
  CREATE TABLE workstreams (
      id TEXT PRIMARY KEY,
      name TEXT NOT NULL,
      root_dir TEXT NOT NULL,
      created_at TEXT NOT NULL
  );
  CREATE TABLE sessions (
      id TEXT PRIMARY KEY,
      workstream_id TEXT,
      created_at TEXT NOT NULL,
      FOREIGN KEY (workstream_id) REFERENCES workstreams(id)
  );
  ```
- `DataLayout` is a declarative description of the expected directory tree. On startup, `ensure()` walks the declaration and creates any missing directories. This is the filesystem equivalent of refinery — a versioned schema for the directory structure. Future versions can add new directories (e.g., `knowledge/`, `watchers/`) without ad-hoc mkdir calls scattered through the code.
- Depends on: nothing (new crate, independent of engine)

## Status Updates
- **2026-03-31**: Complete. arawn-storage crate with rusqlite 0.35 (bundled) + refinery 0.9. V1 migration creates workstreams + sessions tables. Database::open/in_memory with auto-migration. DataLayout::v1() declares workstreams/ + scratch/sessions/ with ensure() for idempotent reconciliation. StorageError with variants for DB/migration/IO/JSON/NotFound/InvalidOperation. 6 tests passing, clippy clean.