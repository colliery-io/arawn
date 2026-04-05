---
id: arawn-memory-crate-entity-types
level: task
title: "arawn-memory crate — entity types, SQLite store, FTS5 search, relations, tags"
short_code: "ARAWN-T-0094"
created_at: 2026-04-05T14:41:46.230603+00:00
updated_at: 2026-04-05T15:16:06.051948+00:00
parent: ARAWN-I-0015
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0015
---

# arawn-memory crate — entity types, SQLite store, FTS5 search, relations, tags

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0015]]

## Objective

New `arawn-memory` crate providing the core knowledge base storage layer. Entity types, SQLite-backed CRUD, FTS5 full-text search, directed relations between entities, and tag support. This is the data foundation everything else builds on.

### Type: Feature | Priority: P1 | Effort: L

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `Entity` struct with id, entity_type, title, content, confidence, reinforcement_count, superseded, tags, source_session, timestamps
- [ ] `EntityType` enum: Fact, Decision, Convention, Preference, Person, Note
- [ ] `RelationType` enum: relates_to, contradicts, supports, supersedes, extracted_from, mentions, belongs_to
- [ ] `Relation` struct with source_id, relation_type, target_id, created_at
- [ ] `ConfidenceSource` enum (Stated/Observed/Inferred) with score computation: base * reinforcement_boost * staleness_decay
- [ ] SQLite store with WAL mode, entity CRUD (insert/get/update/delete), bulk list by type
- [ ] FTS5 virtual table indexing entity titles + content, search returns ranked results
- [ ] Relations table with add/get/delete, get_neighbors for graph traversal
- [ ] Tags stored as JSON array on entity, filterable via SQL
- [ ] `store_fact()` — search-before-create: returns Inserted/Reinforced/Superseded
- [ ] Crate added to workspace with SQLite (rusqlite) dependency
- [ ] Unit tests: CRUD, FTS5 search, relations, tags, confidence scoring, store_fact dedup

## Implementation Notes

- Port core types from backup `arawn-memory/src/types.rs` — simplify to 6 entity types
- Use rusqlite directly (already a dependency via arawn-storage)
- FTS5: `CREATE VIRTUAL TABLE entities_fts USING fts5(title, content, content=entities, content_rowid=rowid)`
- Relations: simple adjacency table, not graphqlite initially (can upgrade later)
- `store_fact` uses FTS5 search + entity_type match to find candidates, then compares titles for reinforce/supersede logic

## Status Updates **[REQUIRED]**

### 2026-04-05
- Created `arawn-memory` crate with types (Entity, EntityType, RelationType, ConfidenceSource, Relation, StoreFactResult)
- SQLite store with WAL mode, entity CRUD, FTS5 virtual table with triggers for auto-sync
- Relations as adjacency table (add/get/delete/get_neighbors)
- Tags stored as JSON array, searchable via json_each
- Confidence scoring: base * reinforcement_boost * staleness_decay
- store_fact with search-before-create: case-insensitive title match → reinforce, supersede_entity marks old + creates supersedes relation
- 24 tests passing: CRUD, FTS5 search, search_by_type, relations, tags, store_fact (insert/reinforce/supersede), confidence scoring