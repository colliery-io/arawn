---
id: 002-memory-on-graphqlite-schema-as-rust
level: adr
title: "arawn-memory on graphqlite: schema lives in Rust, default to Cypher"
number: 2
short_code: "ARAWN-A-0002"
created_at: 2026-05-11T00:00:00+00:00
updated_at: 2026-05-11T00:00:00+00:00
decision_date: 2026-05-11
decision_maker: dstorey

tags:
  - "#adr"
  - "#phase/decided"

initiative_id: ARAWN-I-0040
---

# ARAWN-A-0002: arawn-memory on graphqlite — schema lives in Rust, default to Cypher

## Status

Decided 2026-05-11. Implements Phase 1 of I-0040.

## Context

I-0040 rebuilds `arawn-memory` on graphqlite so that entity/relation storage,
projections, and workstream palaces all sit on the same graph substrate. The
Phase 0 spike established that graphqlite is intentionally schemaless — its
EAV property tables accept any (entity, key, value) tuple. There is no
graphqlite-side schema-management API to wire up.

That left several questions for Phase 1, answered here.

## Decisions

### 1. graphqlite stays schemaless. Schema lives in Rust.

The closed enums `EntityType` (`Fact`, `Decision`, `Convention`, `Preference`,
`Person`, `Note`) and `RelationType` (`RelatesTo`, `Contradicts`, `Supports`,
`Supersedes`, `ExtractedFrom`, `Mentions`, `BelongsTo`) define the entity
ontology. Validation happens at the public `MemoryStore` API boundary, before
any Cypher query is constructed. graphqlite sees only well-formed labels and
property bags.

**Why not push schema into graphqlite?** graphqlite has no schema-management
primitives and adding them is a much larger initiative than I-0040 needs.
Rust-side enforcement is sufficient: the type system rejects invalid types at
compile time, and the conversion layer (`cypher_schema::entity_to_props`) is
the only place where Cypher parameters get assembled.

### 2. Default to Cypher for entity/relation operations.

Where the same query is expressible as either Cypher or raw SQL against
graphqlite's `nodes`/`edges`/`*_props_*` EAV tables, prefer Cypher. The Cypher
layer survives storage-internal changes; raw EAV joins do not.

Exceptions, applied case-by-case in T-0240:

- FTS5 search queries — graphqlite has no full-text primitive; raw SQL against
  the FTS5 virtual table.
- Vector similarity — same reason; raw SQL against the vector extension table.
- Hybrid search-before-create — composes FTS/vector reads (raw SQL) with
  Cypher writes inside a single transaction.

### 3. Tags are stored as a JSON-string property, not multi-label.

Multi-label was tempting (lets us match tags via Cypher `(:Fact:Performance)`)
but creates label explosion: tags are mutable and unbounded, and the steward
subroutines (re-shelve, dust, map) in later phases need to enumerate them.
A schema with hundreds of dynamically-created labels is hostile to the steward
and to anyone reading the graph.

JSON-string keeps tags first-class without polluting the label namespace.
`json_each(tags)` from raw SQL still works for tag-filtered queries when
needed; T-0240 may revisit if Cypher-native tag matching becomes a hot path.

### 4. Single sqlite handle, dual-API access.

`MemoryStore` holds a `graphqlite::Connection` which wraps a single
`rusqlite::Connection`. Cypher goes through `.cypher_builder(...)`; raw SQL
(legacy `entities`/`relations` tables this round; FTS+vector tables next
round) goes through `.sqlite_connection()`. This avoids the two-connection
coordination problem and means a future transaction can span both interfaces.

### 5. Migration cadence — dual-write during Phase 1, single-source after T-0240.

Phase 1 (T-0239) leaves the legacy `entities`/`relations` SQL tables in place
and dual-writes them alongside Cypher. This keeps FTS5 triggers and the
vector index functional through the migration. T-0240 will drop the legacy
SQL tables once FTS/vector colocate against graphqlite.

There is no production data to migrate (per the no-userbase decision in
I-0040), so backward compatibility is not a concern.

## Consequences

**Good.**
- The closed-enum schema gives type-safe validation at the API boundary.
- Cypher queries port if we ever swap graphqlite for another graph engine.
- Steward operations (Phases 4–5) get a stable label vocabulary to walk.
- Tags remain searchable without polluting the label namespace.

**Tradeoffs.**
- Adding a new entity type or relation type requires a Rust code change.
  Acceptable: the ontology is meant to be small and stable.
- Tag-filtered queries fall back to raw SQL (`json_each`) rather than native
  Cypher. Pragmatic given the JSON-property decision.
- Two write paths (SQL + Cypher) during Phase 1 are extra surface area for
  bugs. Mitigated by tests that explicitly delete SQL rows and verify reads
  still pass via Cypher.

## References

- [[ARAWN-I-0040]] — Signal extraction initiative.
- [[ARAWN-T-0238]] — graphqlite wired into workspace.
- [[ARAWN-T-0239]] — MemoryStore CRUD on graphqlite (this ADR's implementation).
- graphqlite v0.4.4 EAV storage docs.
