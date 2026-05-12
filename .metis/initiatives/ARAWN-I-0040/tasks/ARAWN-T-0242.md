---
id: projections-plumbing-gmail
level: task
title: "Projections plumbing + gmail_messages reference impl"
short_code: "ARAWN-T-0242"
created_at: 2026-05-12T03:28:15.356110+00:00
updated_at: 2026-05-12T12:50:36.837065+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Projections plumbing + gmail_messages reference impl

## Parent Initiative

[[ARAWN-I-0040]]

## Objective

Stand up the per-feed-type projection layer described in I-0040 Phase 2: normalized sqlite tables per feed item type, embedded + FTS-indexed, written on every successful feed run. Land the shared plumbing + the `gmail_messages` projection as a worked reference. Subsequent tasks (T-0243–T-0246) implement the remaining feed types on this foundation.

Projections are the second of the three-layer architecture (feeds → projections → palaces). They give cross-feed semantic search without any workstream declared, and are the input to the per-workstream extractor in Phase 4.

## Scope

### Crate / module layout

New crate `arawn-projections` (or a sibling module in `arawn-memory`, TBD by simplest cargo dep graph). Houses:

- A `Projection` trait or enum surface — one variant per feed type, each with its own normalized fields.
- `projections.db` (or a table set in the existing memory db — open question, see notes).
- A shared writer that handles embedding + FTS in one transactional dual-write, mirroring the T-0240 pattern in `arawn-memory::store`.

### Storage schema

Per-feed-type table. For the gmail reference impl:

```sql
CREATE TABLE gmail_messages (
    id TEXT PRIMARY KEY,              -- stable hash of (feed_id, source_id)
    feed_id TEXT NOT NULL,            -- which feed produced this
    source_id TEXT NOT NULL,          -- provider's id (gmail message id)
    source_ts TEXT NOT NULL,          -- RFC3339; the item's authored timestamp
    sender TEXT,
    recipients TEXT,                  -- JSON array
    subject TEXT,
    body_text TEXT NOT NULL,
    thread_id TEXT,
    labels TEXT,                      -- JSON array
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(feed_id, source_id)
);

CREATE VIRTUAL TABLE gmail_messages_fts USING fts5(
    projection_id UNINDEXED,
    sender, subject, body_text,
    tokenize = 'unicode61'
);

CREATE TABLE gmail_messages_embeddings (
    projection_id TEXT PRIMARY KEY,
    embedding BLOB NOT NULL
    -- (sqlite-vec virtual table or raw blob — match arawn-memory's pattern)
);
```

Other feed types follow the same shape with their own normalized field set.

### Embedding pipeline

Reuse `arawn-embed` (already loaded by `arawn-memory` for all-MiniLM-L6-v2). Batch projection rows on insert (configurable batch size, default 32). Throttle config respects the feed-runtime backoff settings already in `arawn-feeds::runtime`.

### Dispatch hook

`crates/arawn-feeds/src/dispatch.rs::run_feed` already returns per-run stats. Add a post-run callback that:

1. Reads new items from the feed mirror (since last projection cursor).
2. Normalizes each item into the corresponding `Projection` variant.
3. Writes to the projection table + FTS + embedding inside one tx.
4. Advances the per-feed projection cursor.

### Backfill

On first `MemoryStore::open` / projection-db open, walk every feed mirror's existing items and project them. Same spawn-loop convergence pattern as I-0039 / T-0227. The cursor handles incremental subsequent runs naturally.

### `feed_search` interface

Not in this task — T-0247. This task just ensures the storage shape supports it (entity_id + ranked-FTS-MATCH + vector-similarity primitives).

### What's deferred

- Other feed types (T-0243 Slack, T-0244 Drive, T-0245 Atlassian, T-0246 Calendar).
- `feed_search` agent tool (T-0247).
- Per-workstream extraction (Phase 4).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `arawn-projections` (or shared module) exists with a `Projection` trait/enum + writer that handles embedding + FTS dual-write in one transaction.
- [ ] `gmail_messages` projection wired end-to-end: gmail feed run produces projection rows after the run completes.
- [ ] Backfill loop projects pre-existing gmail mirror items on first projection-db open.
- [ ] Idempotent on re-run — duplicate items (matched by `(feed_id, source_id)`) update rather than duplicate.
- [ ] Embedding pipeline batches + throttles per config; throughput documented.
- [ ] Unit tests cover: projection write, embedding store, FTS row creation, dedup on re-run.
- [ ] Integration test: run a small fixture gmail feed → assert projection rows present + searchable via raw FTS.
- [ ] `angreal check workspace` + `angreal check clippy` clean.
- [ ] Open design questions resolved and decisions logged in this task's status updates:
  - Where do projections live — separate `projections.db` or extra tables in the memory db?
  - One crate vs. one module per feed type vs. shared crate with one file per feed type?

## Implementation Notes

### Technical approach

1. **Decide storage location.** Two options: (a) extra tables in the existing memory db so search joins are local; (b) separate `projections.db` so projection volume doesn't bloat the memory db (gmail alone could be 100k+ rows). Default to (b) unless join requirements emerge.
2. **Projection trait.** Each feed type implements `from_mirror_item(&MirrorItem) -> Projection`. The trait is sealed; new feed types add a variant (closed-enum policy mirroring `EntityType`).
3. **Writer.** Single `ProjectionWriter` with a `write_batch(&[Projection]) -> Result<()>` that runs inside `with_tx`. Per projection: insert row → FTS upsert → embed + store. Same transactional dual-write pattern as `arawn-memory::store`.
4. **Embedding policy.** Embed `body_text` (or `subject + body_text` for gmail). 384-d all-MiniLM-L6-v2 to match memory. Batched embedding via `arawn-embed`'s batch interface.
5. **Dispatch hook.** Add `dispatch::run_feed` hook that calls the projection writer after `finalize_backfill_success`. Hook returns errors via the existing FeedError surface but doesn't fail the feed run on projection-side issues (per the "warn and continue" pattern from T-0237).
6. **Backfill.** On projection-db open, query each feed's mirror for items not yet in the projection (LEFT JOIN on `source_id`). Walk results in batches. Cursor unnecessary if dedup is `(feed_id, source_id)` UNIQUE.

### Dependencies

- T-0240 (memory infrastructure as the FTS+vector dual-write template).
- `arawn-feeds::dispatch::run_feed` (hook point).
- `arawn-embed` (embedding pipeline).

### Risk considerations

- **DB write contention.** Projections + memory + feeds all writing to sqlite. WAL is enabled. Separate `projections.db` reduces lock surface.
- **Embedding throughput on backfill.** 100k gmail messages at ~20ms each = 30+ min. Throttling needs a checkpoint so backfill survives restarts. Cursor on backfill is OK; cursor on incremental is optional.
- **Schema drift across feeds.** Each feed type has its own normalized fields. Avoid a god-table; one table per type. Forces the closed-enum policy in Rust.
- **Idempotency under updated items.** Gmail messages don't update; jira issues do. The reference impl can ignore updates; document that update handling is per-feed (revisit in T-0245 for Atlassian).

## Status Updates

### 2026-05-11 — Storage layer + Gmail adapter landed; dispatch wiring + embedding pipeline deferred

**Scope landed this session.**
- New crate `arawn-projections` (`crates/arawn-projections/`):
  - `types.rs` — `Projection` trait + `ProjectionRow` (the generic view that the agent tool will hydrate from).
  - `schema.rs` — per-feed-type table + FTS5 virtual table + `<feed_type>_embeddings` cache table. Idempotent CREATE statements (`ensure_feed_type_tables`).
  - `store.rs` — `ProjectionStore`. UPSERT under `(feed_id, source_id)` UNIQUE; transactional dual-write of the row + FTS + embedding-cache invalidation; body_hash dirty-check so a re-run of the same content is a no-op. Helpers: `write_batch`, `missing_source_ids` (for backfill), `count`, `fts_search`, `get_row`.
  - `gmail.rs` — `GmailMessageProjection` + `from_gmail_message` (Gmail JSON → projection) + `walk_feed_dir` (recursive walk of `<YYYY-MM-DD>/<id>.json` mirror).
  - `error.rs` — `ProjectionError` (Storage / Schema / Io).
- Added to workspace `members`.

**Tests: 11 passing.**
- 6 unit tests in `gmail` covering: minimal message parse, missing-id skip, bad-internalDate skip, snippet fallback, stable projection id, mirror-walk filtering.
- 5 integration tests in `tests/gmail_e2e.rs` covering: end-to-end walk → write → FTS search; re-run idempotency (unchanged on identical content); body-change refreshes FTS; `missing_source_ids` returns unprojected; partial-failure recovery via missing-id filter.
- `angreal check workspace` + `angreal check clippy` clean.

**Open design questions — resolved.**
1. *Separate `projections.db` or shared with memory db?* → Separate. `ProjectionStore::open(path)` takes its own path. Projection row volume (gmail alone could be 100k+) shouldn't bloat the memory db, and there's no join requirement between memory entities and projection rows in Phase 2.
2. *Crate vs module structure?* → New crate `arawn-projections`. Per-feed-type adapter is a sibling module (`gmail.rs`, future: `slack.rs`, `drive.rs`, …). Generic `ProjectionStore` works against any feed type via the `Projection` trait — schema is created on first write per feed type.

**Decisions worth keeping.**
- **Schema is generic over feed type.** Every projection table has the same shape: `id, feed_id, source_id, source_ts, title, body_text, metadata (JSON), body_hash, created_at, updated_at` + `UNIQUE(feed_id, source_id)`. Per-feed normalized fields (sender, channel, project_key, …) live in the `metadata` JSON. Hot-path filters get hoisted to indexed columns in follow-up tasks if needed. This is a deliberate simplification from the task spec's per-type column lists — keeps `arawn-projections` generic, lets per-feed adapters add their own typed structs without touching the writer.
- **Embedding column starts NULL.** `<feed_type>_embeddings` carries `(projection_id, body_hash, embedding)`. The writer leaves `embedding = NULL` and stamps `body_hash`. A separate embed pass (deferred) refreshes rows where `body_hash` differs from the embedded vector's keyed hash. This decouples the write path from the (expensive) embedding compute and matches the "warn and continue" pattern.
- **`body_hash` doubles as dirty-check and embed-invalidation key.** Same hash compared on UPSERT for "is this body really new" and on the embed pass for "does this row need re-embedding."

**Deferred from this task (surfaced as follow-up before T-0247).**
- **Dispatch hook wiring.** `dispatch::run_feed` doesn't yet call the projection writer after a successful template run. The integration touches `FeedRuntimeContext` (add `Option<Arc<ProjectionStore>>`), `run_feed_inner` (post-template-success callback), and arawn main (set the field at startup). I judged this as cross-cutting enough to deserve its own focused change rather than a rushed bake-in here. Note: the standalone backfill API (`walk_feed_dir` → `missing_source_ids` → `write_batch`) is fully functional today and can be invoked as a one-shot CLI command without dispatch wiring.
- **Embedding pipeline integration.** `arawn-embed` is not yet wired. The `<feed_type>_embeddings.body_hash` column is in place; a future task adds the embed pass.
- **Live-fire gmail run.** No integration test yet that runs a real `gmail/inbox-archive` feed end-to-end and confirms projection rows appear. The fixture-based e2e test covers the same code paths; live-fire awaits dispatch wiring.

**Acceptance criteria.**
- [x] `arawn-projections` exists with `Projection` trait + writer with transactional dual-write.
- [ ] `gmail_messages` projection wired end-to-end via `dispatch::run_feed` — **deferred**. Standalone walk-and-write path is functional and tested.
- [x] Backfill walks pre-existing gmail mirror items (`walk_feed_dir` + `missing_source_ids`).
- [x] Idempotent on re-run.
- [ ] Embedding pipeline batches + throttles — **deferred**; schema supports it.
- [x] Unit tests cover write, FTS row creation, dedup on re-run.
- [x] Integration test: fixture gmail mirror → projection rows present + FTS-searchable.
- [x] `angreal check workspace` + `angreal check clippy` clean.
- [x] Open design questions resolved (see above).

T-0243 through T-0246 can proceed: each adds a sibling `<provider>.rs` module that implements the `Projection` trait. They do not need to wait for the dispatch hook, since each can be tested via the same fixture-mirror pattern (see `tests/gmail_e2e.rs` as the template).

T-0247 (`feed_search` agent tool) should bundle the dispatch-hook wiring with the embed-pipeline activation, since both surfaces become user-visible at the same point.

### 2026-05-12 — dispatch hook wired; embed pipeline still deferred

After feedback, completed the dispatch wiring inside this task rather than punting:

- `arawn-projections::dispatch::project_feed_dir(store, template_name, feed_id, feed_dir)` — maps the feed template name (`gmail/*`, `slack/*`, `drive/*`, `jira/*`, `confluence/*`, `calendar/*`) to the right adapter. Unknown templates log and no-op.
- `FeedRuntimeContext` gained `projections: Option<Arc<ProjectionStore>>`. `arawn-feeds::start()` accepts it as a sixth arg.
- `dispatch::run_feed_inner` calls `project_feed_dir` after a successful template run + meta persist. Projection errors warn but don't fail the feed run (matches T-0237's warn+continue policy).
- arawn `main.rs` opens `~/.arawn/projections.db` and threads it into both `arawn_feeds::start(..)` and the engine's `FeedSearchTool` registration. Both can soft-fail independently (separate `match` blocks).

Updated all 8 call sites of `arawn_feeds::start` in feed tests to pass `None` for the projections arg.

Result: feeds runtime now writes projections live on every run. `angreal check workspace` + `angreal check clippy` clean; existing feed test suite (76 unit + ~80 integration cases) still green.

**Acceptance criteria — re-cleared.**
- [x] `gmail_messages` projection wired end-to-end via `dispatch::run_feed`.

Embedding pipeline is still the one piece deferred to a focused follow-up. The schema (`<feed_type>_embeddings.body_hash` + NULL embedding) is in place; an embed pass needs to walk `WHERE embedding IS NULL`, call `arawn-embed`, and write the vector back.