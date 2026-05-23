---
id: persistence-sqlite-metadata-jsonl
level: initiative
title: "Persistence — SQLite metadata + JSONL message storage"
short_code: "ARAWN-I-0002"
created_at: 2026-03-31T22:34:00.187767+00:00
updated_at: 2026-04-02T12:35:41.235681+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: persistence-sqlite-metadata-jsonl
---

# Persistence — SQLite metadata + JSONL message storage Initiative

## Context

Everything is in-memory right now — sessions and messages die when the process exits. To be usable day-to-day, Arawn needs durable storage that survives across runs.

The storage model is split by access pattern:
- **SQLite** for operational metadata (workstreams, sessions) — fast queries, structured, small rows
- **JSONL files** for message content — append-heavy, potentially large, streamable, human-inspectable

This separation keeps the database lean and the conversation data easy to debug/backup.

### Reference
- ARAWN-I-0001: established the domain types (Workstream, Session, Message) as in-memory objects
- Vision doc: "Persists state in SQLite" + "filesystem-level isolation"

## Goals & Non-Goals

**Goals:**
- Persist workstreams and session metadata in SQLite
- Persist messages as JSONL files on disk, organized by workstream
- Database migrations via refinery
- Session promotion moves JSONL from scratch/ to target workstream/
- Engine and tools work transparently with persisted sessions
- New `arawn-storage` crate owning all persistence logic

**Non-Goals:**
- Graphqlite/knowledge graph (separate initiative later)
- Full-text search over messages
- Message encryption at rest
- Remote/cloud storage

## Architecture

### Filesystem Layout

```
~/.arawn/                                 # Default, configurable via arawn.toml
├── arawn.db                              # SQLite: workstreams, sessions metadata
├── workstreams/
│   ├── <ws-uuid>/
│   │   └── sessions/
│   │       ├── <session-uuid>.jsonl      # One JSON object per message
│   │       └── ...
│   └── ...
└── scratch/
    └── sessions/
        └── <session-uuid>.jsonl
```

### Storage Traits

```rust
// arawn-storage or arawn-core
#[async_trait]
trait WorkstreamStore {
    async fn create(&self, ws: &Workstream) -> Result<()>;
    async fn get(&self, id: Uuid) -> Result<Option<Workstream>>;
    async fn list(&self) -> Result<Vec<Workstream>>;
    async fn delete(&self, id: Uuid) -> Result<()>;
}

#[async_trait]
trait SessionStore {
    async fn create(&self, session: &Session) -> Result<()>;
    async fn get(&self, id: Uuid) -> Result<Option<Session>>;
    async fn list_for_workstream(&self, ws_id: Uuid) -> Result<Vec<Session>>;
    async fn list_scratch(&self) -> Result<Vec<Session>>;
    async fn promote(&self, session_id: Uuid, new_ws_id: Uuid) -> Result<()>;
}

#[async_trait]
trait MessageStore {
    async fn append(&self, session_id: Uuid, msg: &Message) -> Result<()>;
    async fn load(&self, session_id: Uuid) -> Result<Vec<Message>>;
}
```

SQLite implements `WorkstreamStore` + `SessionStore`. Filesystem implements `MessageStore` (JSONL read/write).

### SQLite Schema (V1)

```sql
CREATE TABLE workstreams (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    root_dir TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    workstream_id TEXT,  -- NULL = scratch
    created_at TEXT NOT NULL,
    FOREIGN KEY (workstream_id) REFERENCES workstreams(id)
);
```

### Migrations

Using **refinery** for versioned SQL migrations. Migrations live in `crates/arawn-storage/migrations/` as numbered SQL files (`V1__initial.sql`, etc.). Run automatically on startup.

## Detailed Design

### arawn-storage crate

New crate depending on `arawn-core` (for domain types). Dependencies: `rusqlite` (with bundled SQLite), `refinery` (migrations), `tokio` (async fs for JSONL).

- `sqlite.rs` — SQLite connection pool, implements WorkstreamStore + SessionStore
- `jsonl.rs` — JSONL file reader/writer, implements MessageStore  
- `migrations/` — refinery SQL migration files
- `store.rs` — `Store` struct that composes SQLite + JSONL behind a unified interface

### JSONL Format

One JSON object per line, matching `arawn_core::Message` serialization:
```jsonl
{"role":"user","content":"List the crates"}
{"role":"assistant","content":"","tool_uses":[{"id":"c1","name":"shell","input":{"command":"ls crates/"}}]}
{"role":"tool_result","tool_use_id":"c1","content":"arawn\narawn-core\n...","is_error":false}
{"role":"assistant","content":"Here are the crates: ..."}
```

### Session Promotion

`promote(session_id, new_ws_id)`:
1. Update `sessions.workstream_id` in SQLite
2. Move JSONL file from `scratch/sessions/<id>.jsonl` to `workstreams/<ws_id>/sessions/<id>.jsonl`
3. Both in a single logical operation (SQLite update + fs move)

### Integration with Engine

The `QueryEngine` currently takes `&mut Session` with in-memory messages. After this initiative:
- On engine start: load session from store (metadata from SQLite, messages from JSONL)
- After each tool result / assistant response: append to JSONL via `MessageStore::append`
- Session object still holds in-memory messages for the engine loop, but they're also durably written

## Alternatives Considered

1. **All in SQLite (messages as rows)** — Rejected. Message content can be large (tool outputs), and append-heavy writes to SQLite are slower than JSONL. Also harder to inspect/debug.
2. **All on filesystem (no SQLite)** — Rejected. Querying "list all workstreams" or "find sessions by date" requires scanning directories. SQLite gives us indexed queries.
3. **sled/redb** — Rejected. SQLite is battle-tested, has great Rust bindings, and the vision doc specifies it. No reason to deviate.
4. **diesel for migrations** — Rejected. refinery is lighter, SQL-file based, and doesn't require a full ORM. Better fit for our needs.

## Implementation Plan

Tasks will be decomposed after design approval. Rough ordering:
1. Create `arawn-storage` crate with SQLite + refinery setup + V1 migration
2. Implement WorkstreamStore + SessionStore on SQLite
3. Implement MessageStore (JSONL read/append)
4. Compose into unified Store
5. Wire into binary crate — auto-create ~/.arawn/, run migrations, load/save sessions
6. Session promotion (SQLite update + file move)
7. Tests for all of the above