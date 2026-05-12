---
id: memorystore-crud-on-graphqlite-via
level: task
title: "MemoryStore CRUD on graphqlite via Cypher (entities + relations)"
short_code: "ARAWN-T-0239"
created_at: 2026-05-12T01:33:02.233952+00:00
updated_at: 2026-05-12T01:33:02.233952+00:00
parent: ARAWN-I-0040
blocked_by: ["ARAWN-T-0238"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

*To be added during implementation*
