---
id: memorystore-crud-on-graphqlite-via
level: task
title: "MemoryStore CRUD on graphqlite via Cypher (entities + relations)"
short_code: "ARAWN-T-0239"
created_at: 2026-05-12T01:33:02.233952+00:00
updated_at: 2026-05-12T03:25:31.650801+00:00
parent: ARAWN-I-0040
blocked_by: [ARAWN-T-0238]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# MemoryStore CRUD on graphqlite via Cypher (entities + relations)

## Parent Initiative

[[ARAWN-I-0040]]

## Objective

Rewrite the core CRUD path of `MemoryStore` so that `Entity` and `Relation` records flow through graphqlite Cypher queries instead of the current hand-rolled sqlite tables. The schema lives in Rust types (closed enums for `EntityType` + `RelationType`); validation happens at the Rust public API boundary. graphqlite stays schemaless — its EAV storage is sufficient.

This task does NOT touch FTS5 / vector indexes or the search-before-create dedup path — those are T-0240. The goal here is: every `MemoryStore` method that today reads/writes the entity or relation tables instead reads/writes via Cypher, and all the tests of that direct CRUD surface pass.

## Scope

### Schema mapping (Entity ↔ Cypher)

- Every `Entity` becomes a Cypher node with one label (its `EntityType`, capitalized: `Fact`, `Decision`, `Convention`, `Preference`, `Person`, `Note`).
- Node properties carry the Entity's scalar fields: `id` (uuid as text), `title`, `content`, `confidence_source` (stated/observed/inferred), `reinforcement_count`, `superseded`, `source_session`, `created_at`, `updated_at`, `accessed_at`.
- `tags: Vec<String>` becomes a JSON-typed property. Multi-label was the alternative but tags are mutable and unbounded; multi-label would create a label-explosion problem for the steward in palace workstreams later. JSON-property keeps tags first-class but contained.

### Schema mapping (Relation ↔ Cypher)

- Every `Relation` becomes a typed Cypher edge: `(source)-[r:RELATION_TYPE]->(target)`, where `RELATION_TYPE` is the screaming-snake form of `RelationType` (`RELATES_TO`, `CONTRADICTS`, `SUPPORTS`, `SUPERSEDES`, `EXTRACTED_FROM`, `MENTIONS`, `BELONGS_TO`).
- Edge property: `created_at`.

### MemoryStore rewrite

- `insert_entity(&Entity)` → `MERGE (n:<Type> {id: $id}) SET n += $props` via `cypher_builder`.
- `get_entity(id)` → `MATCH (n {id: $id}) RETURN n`.
- `update_entity(...)` → `MATCH (n {id: $id}) SET ...`.
- `delete_entity(id)` → `MATCH (n {id: $id}) DETACH DELETE n`.
- `insert_relation(&Relation)` → `MATCH (a {id: $src}), (b {id: $tgt}) MERGE (a)-[r:TYPE]->(b) SET r.created_at = $ts`.
- `get_relations_for(entity_id, direction)` → `MATCH (n {id: $id})-[r]->(m) RETURN m, type(r)` (and inverse).
- List/filter operations → straightforward Cypher MATCH with WHERE.

Use `Connection.cypher_builder` for parameterized queries everywhere — never string-format user data into Cypher.

### Public API stability

The Rust signatures of `MemoryStore::insert_entity`, `MemoryStore::get_entity`, etc. stay the same. Callers (engine tools `memory_store` / `memory_search`, auto-memory, the MemoryManager) compile and run unchanged. This is the contract that keeps T-0241's tests meaningful.

### What's deferred to T-0240

- FTS5 virtual table and `MemoryStore::search` (text search path).
- Vector index and embedding similarity.
- `store_fact` (search-before-create dedup) — that's the hybrid path that needs both Cypher and the search tables and is the load-bearing case for T-0240.

### What's deferred entirely

- LongMemEval bench — T-0241.
- Migration of existing memory DBs — out of scope per the no-userbase decision in I-0040.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `MemoryStore` opens a graphqlite-backed DB on construction (loads the extension via the `Graph` API or `Connection`).
- [ ] All Entity CRUD methods write through Cypher; all Relation CRUD methods write through Cypher.
- [ ] All Entity/Relation CRUD methods read through Cypher.
- [ ] Schema enforcement (valid `EntityType` / `RelationType`, required fields) happens in Rust before the Cypher query is issued; invalid inputs return `MemoryError` without touching the DB.
- [ ] Existing unit tests covering Entity/Relation CRUD pass.
- [ ] `angreal check workspace` + `angreal check clippy` clean.
- [ ] An ADR drafted at `.metis/adrs/ARAWN-A-0002.md` captures: graphqlite stays schemaless; schema lives in Rust types; default to Cypher; the closed enum decision; the tags-as-JSON-property decision.

## Implementation Notes

### Technical approach

1. Add a `graphqlite::Graph` field to `MemoryStore` (or wrap the existing rusqlite `Connection` such that both can coexist — graphqlite is a sqlite *extension*, so a single `Connection` works for both).
2. The schema enforcement layer is a thin Rust module:
   - `fn entity_label(t: EntityType) -> &'static str`
   - `fn relation_type(t: RelationType) -> &'static str`
   - `fn entity_to_props(e: &Entity) -> Vec<(&'static str, PropertyValue)>`
   - `fn props_to_entity(props: &CypherRow) -> Result<Entity>`
3. CRUD methods become Cypher templates with `cypher_builder` parameter binding.
4. Tests: update existing `MemoryStore` tests to assert Cypher-backed reads + writes round-trip correctly. The semantics shouldn't change — just the storage.

### Dependencies

- T-0238 (graphqlite wired into workspace).

### Risk considerations

- **`MERGE` semantics on properties.** `MERGE (n:Type {id: $id})` matches on `id` and creates if missing — but `MERGE (n {props_all})` will create on any property mismatch. Always match on `id` only, then `SET` the rest.
- **Cypher injection.** Always use `cypher_builder` parameter binding for user-controlled values. Especially `tags` (user-supplied strings); since we go with JSON-property storage, serialize via `serde_json` and pass as a single param.
- **Tags-as-property vs. multi-label.** If we later regret the JSON-property decision (e.g. we want to MATCH on tags via Cypher native ops), we can migrate to multi-label or to a separate `:Tag` node pattern. Both reversible.
- **Confidence scoring.** Today `Entity::confidence_score()` is computed in Rust at read time from `confidence_source`, `reinforcement_count`, `superseded`, and `updated_at`. That stays the same — it's a Rust method on an `Entity`, not a stored field. No DB changes for the score itself.

## Status Updates

### 2026-05-11 — CRUD on graphqlite (dual-write writes, Cypher-only reads)

**Approach.** Rather than break FTS/vector tests in the intermediate state (which T-0240 will retire), CRUD writes dual-write the legacy `entities`/`relations` SQL tables alongside Cypher. CRUD **reads** are exclusively Cypher. T-0240 will retire the SQL writes when FTS+vector colocate against graphqlite.

**Files.**
- `crates/arawn-memory/src/cypher_schema.rs` — new. Closed-enum ↔ Cypher label/edge mapping, Entity ↔ JSON-property projection, `node_to_entity` parser for `MATCH … RETURN n` shape.
- `crates/arawn-memory/src/store.rs` — `MemoryStore.conn: Mutex<graphqlite::Connection>` (single sqlite handle, both APIs). All CRUD methods rewritten; FTS/vector code uses `.sqlite_connection()` against the same handle.
- `crates/arawn-memory/src/lib.rs` — `pub mod cypher_schema`.
- `.metis/adrs/ARAWN-A-0002.md` — new ADR per the task's acceptance criterion.

**Schema mapping summary.**
- Label per entity type: `Fact`, `Decision`, `Convention`, `Preference`, `Person`, `Note`.
- Edge type per relation: `RELATES_TO`, `CONTRADICTS`, `SUPPORTS`, `SUPERSEDES`, `EXTRACTED_FROM`, `MENTIONS`, `BELONGS_TO`.
- Tags stored as JSON-string property (chose over multi-label — see ADR rationale).
- Datetimes as RFC3339 strings; booleans as Cypher bool; counts as i64.

**CRUD methods now on Cypher.**
- Reads: `get_entity`, `list_by_type`, `count_by_type`, `count_all`, `get_relations`, `get_neighbors` → Cypher.
- Writes: `insert_entity`, `update_entity`, `delete_entity` (with `DETACH DELETE`), `add_relation`, `delete_relation`, `reinforce_entity` (counter mirrored), `supersede_entity` → dual-write.
- `list_all_ranked` → Cypher fetch + sort in Rust (graphqlite rejected `CASE` in `ORDER BY`; see Findings).

**Untouched (per task — T-0240 territory).** `search`, `search_by_type`, `search_by_tags`, `store_fact`, vector ops. All routed via `conn.sqlite_connection()` so they continue working against legacy tables this round.

**Cypher emulation choices.**
- Used explicit existence-check + CREATE / SET split rather than `MERGE … SET n += $props`. graphqlite's MERGE has constraints we'd rather avoid pinning down here; the explicit pattern matches what `Graph::upsert_node` does internally.
- Always `cypher_builder().param(...)` for user data. Labels and edge types come from closed Rust enums and are interpolated into the query string — safe.

**Findings.**
1. **graphqlite's Cypher does not accept `CASE` in `RETURN`/`ORDER BY`** — surfaced when `list_all_ranked` tried `(CASE n.confidence_source WHEN 'stated' THEN 3 …)`. Worked around by Rust-side sorting after a single `MATCH (n) WHERE n.superseded = false RETURN n`. Worth filing upstream against graphqlite; acceptable workaround at memory-scale corpora.
2. **`Connection::sqlite_connection()` is the right escape hatch** for FTS/vector. One handle, two APIs, no transaction-coordination headaches.

**Tests.**
- `cargo test -p arawn-memory --lib`: **62 passed**, 0 failed (3-of-3 consecutive runs green).
- `cargo test -p arawn-memory --test recall_eval`: **8 passed**, 0 failed.
- `cargo test -p arawn-tests` (engine-level memory_tools, memory_stack): all green, no source changes — public API stayed stable.
- New `store::tests` that prove reads are Cypher-backed: delete the legacy SQL rows under the store, then assert `get_entity` / `count_*` / `list_*` / relations still return the data (only the Cypher path can satisfy them).
- New `cypher_schema::tests` cover label/edge roundtrip and tags-as-JSON-string serialization.
- `angreal check workspace` + `angreal check clippy` clean.

**Acceptance criteria.**
- [x] `MemoryStore` opens a graphqlite-backed DB on construction.
- [x] All Entity CRUD writes dual-write through Cypher; Relation CRUD writes likewise.
- [x] All Entity/Relation CRUD reads go through Cypher.
- [x] Schema enforcement at the Rust public API boundary (closed enums + `entity_to_props`).
- [x] Existing CRUD unit tests pass (and so do the FTS/vector/store_fact/supersede tests by virtue of dual-write).
- [x] `angreal check workspace` + `angreal check clippy` clean.
- [x] ADR `ARAWN-A-0002.md` drafted.

T-0240 unblocked. The legacy SQL tables remain populated this round so FTS triggers and vector index continue working; T-0240 will retire them when FTS5+vector colocate against the graphqlite tables.