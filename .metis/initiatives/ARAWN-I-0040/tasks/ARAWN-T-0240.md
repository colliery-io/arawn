---
id: fts5-vector-indexes-colocated-with
level: task
title: "FTS5 + vector indexes colocated with graphqlite; hybrid search-before-create dedup"
short_code: "ARAWN-T-0240"
created_at: 2026-05-12T01:33:03.386272+00:00
updated_at: 2026-05-12T01:33:03.386272+00:00
parent: ARAWN-I-0040
blocked_by: ["ARAWN-T-0239"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# FTS5 + vector indexes colocated with graphqlite; hybrid search-before-create dedup

## Parent Initiative

[[ARAWN-I-0040]]

## Objective

graphqlite doesn't ship FTS or vector search. arawn-memory does both today via FTS5 virtual tables and a vector extension. After T-0239 the entity data lives in graphqlite's EAV tables; the FTS and vector tables need to come along into the same sqlite DB, kept in sync with entity writes, and used by `MemoryStore::search` and `MemoryStore::store_fact` (the search-before-create dedup).

This task finishes the storage migration started in T-0239: the public search + dedup APIs work end-to-end on the new backend.

## Scope

### Colocate FTS + vector tables

- Create FTS5 virtual table (and rowid mapping) and the vector extension's table in the *same* sqlite DB graphqlite is using. graphqlite is a sqlite extension on a normal sqlite DB, so the FTS5 + vector tables sit alongside graphqlite's `nodes`, `edges`, `node_props_*`, etc. tables in the same file.
- Schema for FTS row: stable rowid keyed by Entity `id`; columns for the text fields we currently search (title, content, possibly tags). Mirror the schema that exists today.
- Schema for vector row: same — Entity `id` keyed, embedding blob.

### Keep them in sync with entity writes

Every entity insert / update / delete needs to touch the FTS + vector tables alongside the Cypher write. Two patterns to choose from:

- **Same-transaction dual-write** in Rust: `MemoryStore::insert_entity` runs the Cypher MERGE, then the FTS upsert, then the vector upsert, all within a single sqlite transaction.
- **SQLite triggers** on graphqlite's underlying tables: triggers fire to maintain the FTS + vector tables. Probably brittle because graphqlite's tables are an implementation detail; T-0239's MERGE may not produce simple INSERT/UPDATE statements at the SQL level.

**Recommend the dual-write-in-Rust pattern.** Explicit, debuggable, no coupling to graphqlite's internal tables.

### `MemoryStore::search` (read path)

The text + semantic search path lives on the FTS + vector tables, not Cypher:

- Text query: FTS5 MATCH against the search index, returning Entity `id`s with rank.
- Semantic query: vector similarity against the embedding column, returning Entity `id`s with score.
- Hybrid: union + rerank.

Then for each result, fetch the full Entity via Cypher (`MATCH (n {id: $id}) RETURN n`), preserving the T-0239 schema-enforcement layer. The result type stays the same as today — engine tool callers don't see any change.

### `store_fact` (search-before-create dedup)

This is the load-bearing hybrid path:

1. Run a search (FTS + vector) on the candidate fact against existing entities of the same type and scope.
2. If a near-duplicate exists above the similarity threshold → reinforce (Cypher SET `reinforcement_count = reinforcement_count + 1`, `accessed_at = now()`).
3. If a contradicting entity exists → supersede (Cypher SET old.superseded = true, insert new via T-0239's `insert_entity`, then `(new)-[:SUPERSEDES]->(old)`).
4. Otherwise insert via `insert_entity`.

Returns `StoreFactResult::{Inserted, Reinforced, Superseded}` unchanged.

### What's deferred

- LongMemEval bench tuning — T-0241. We expect bench parity but don't gate this task on it; semantic deltas surface in T-0241's analysis.

## Acceptance Criteria

- [ ] FTS5 virtual table + vector extension table live in the same sqlite DB as graphqlite's tables; created on `MemoryStore::open`.
- [ ] Every entity insert/update/delete updates the FTS + vector rows in the same sqlite transaction. Failure of either rolls back the whole operation.
- [ ] `MemoryStore::search` returns the same shape it does today; results are correct for both text and semantic queries.
- [ ] `MemoryStore::store_fact` produces `Inserted` / `Reinforced` / `Superseded` results consistent with today's behavior on a fixed test corpus.
- [ ] Existing unit + integration tests in `crates/arawn-memory/tests/` pass.
- [ ] `recall_eval.rs` (the small recall sanity check in arawn-memory) passes.
- [ ] `angreal check workspace` + `angreal check clippy` clean.

## Implementation Notes

### Technical approach

1. Extend `MemoryStore` constructor to issue the `CREATE VIRTUAL TABLE … USING fts5(…)` and the vector-extension CREATE statements after graphqlite extension load. Idempotent (`IF NOT EXISTS`).
2. Refactor entity write methods from T-0239 to bracket their Cypher write with FTS + vector upserts inside a single sqlite transaction.
3. Reimplement `MemoryStore::search` against the FTS + vector tables, then load full entities via Cypher.
4. Reimplement `store_fact` as the documented 4-step hybrid: search → branch → reinforce / supersede / insert. Reuse T-0239's `insert_entity` for the insert path.
5. Run `recall_eval.rs` to sanity-check that the same input corpus produces the same dedup decisions.

### Dependencies

- T-0239 (entity/relation CRUD via Cypher).

### Risk considerations

- **Transaction scope across graphqlite + raw SQL.** A single sqlite transaction must encompass both the Cypher write (which graphqlite executes via SQL under the hood) and the FTS/vector writes. Confirm graphqlite's Cypher API honors an outer transaction; if not, drop down to `Connection.execute_cypher` plus raw `BEGIN; … COMMIT;`.
- **Reinforcement counter races.** `SET n.reinforcement_count = n.reinforcement_count + 1` inside a transaction is fine for a single process; we don't have multi-writer concerns today.
- **Vector extension load order.** Must happen after graphqlite's extension load, before the first vector query. Confirm both extensions coexist on the same sqlite handle.
- **FTS5 tokenizer choice.** Today's tokenizer (porter? unicode61?) should stay the same to preserve search behavior for T-0241's bench parity.

## Status Updates

*To be added during implementation*
