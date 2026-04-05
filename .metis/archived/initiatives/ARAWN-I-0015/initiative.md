---
id: knowledge-base-memory-system-graph
level: initiative
title: "Knowledge base memory system — graph-backed, two-tier (global + workstream) with extraction pipeline"
short_code: "ARAWN-I-0015"
created_at: 2026-04-05T13:39:10.724978+00:00
updated_at: 2026-04-05T16:08:13.358828+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: XL
initiative_id: knowledge-base-memory-system-graph
---

# Knowledge base memory system — graph-backed, two-tier (global + workstream) with extraction pipeline Initiative

## Context

Arawn has no cross-session memory. Every session starts from zero — the agent doesn't remember user preferences, project conventions, past decisions, or accumulated knowledge. Sessions store messages in JSONL files but nothing persists as structured knowledge.

The backup codebase had a full `arawn-memory` crate (SQLite + graph + vectors) that was never ported. The Clotho project provides a production reference for graph-backed knowledge management via a Claude Code plugin with extraction pipelines, ontology-driven routing, and search-before-create deduplication.

### What Exists Today
- `arawn-storage`: SQLite sessions/workstreams + JSONL message persistence
- No memory crate, no graph, no vector search, no cross-session recall
- Session `memory.md` concept exists in plan mode but not generalized

### Reference Implementations
- **arawn-memory (backup)**: Graph (graphqlite) + vectors (sqlite-vec) + confidence scoring + citation tracking + semantic recall with composite scoring
- **Clotho**: 16 entity types, 12 relation types, ontology-driven extraction, search-before-create, processing log, hooks for session injection, MCP-exposed tools

## Goals & Non-Goals

**Goals:**
- **Two-tier knowledge base**: Global (user-level) + Workstream (project-level) scoped stores
- **Graph-backed storage**: Typed entities with relations, enabling link traversal and pattern discovery
- **Extraction pipeline**: Automatic fact extraction on compaction + explicit `/remember` command
- **Session injection**: Load relevant KB context at session start (global prefs + workstream knowledge)
- **Search-before-create**: Never duplicate — link to existing entities, frequency = signal strength
- **Memory tools**: Agent can query, store, and link knowledge via tools
- **Session scratchpad**: `memory.md` in working dir for ephemeral session notes

**Non-Goals:**
- Full Clotho feature parity (programs, responsibilities, cadences, transcripts)
- Vector/embedding search deferred to later phases (not needed for MVP, but architecture supports it)
- MCP server exposure (memory is internal to arawn, not a separate service)
- Real-time entity extraction from every message (extraction on compaction + explicit triggers)
- Multi-user or collaborative memory

## Architecture

### Storage Tiers

```
~/.arawn/
├── memory.db                           # Global KB (user prefs, cross-project facts)
├── workstreams/
│   ├── {ws-name}-{uuid}/
│   │   ├── memory.db                   # Workstream KB (project knowledge)
│   │   └── ...
│   └── ...
└── ...

{session working_dir}/
└── memory.md                           # Session scratchpad (ephemeral)
```

Each `memory.db` is a self-contained SQLite database with:
- **entities table**: Typed entities with metadata
- **relations table**: Directed edges between entities (via graphqlite or simple table)
- **search index**: FTS5 for text search across entity titles and content

### Entity Types

Simplified from Clotho, focused on what arawn needs:

| Type | Scope | Purpose | Example |
|------|-------|---------|---------|
| `Fact` | Global/WS | Extracted knowledge | "User prefers inline tests" |
| `Decision` | WS | Choices made | "Went with observation-vs-action for plan mode" |
| `Convention` | WS | Patterns/rules | "Commit messages use imperative mood" |
| `Preference` | Global | User preferences | "Terse responses, no trailing summaries" |
| `Person` | Global/WS | Team members | "Alice — backend lead, owns auth service" |
| `Note` | WS | Freeform annotations | "PR #42 needs follow-up after deploy" |

### Relation Types

| Type | Meaning | Example |
|------|---------|---------|
| `relates_to` | General connection | Decision → Convention |
| `contradicts` | Conflicting info | OldFact ⊗ NewFact |
| `supports` | Evidence | Fact → Decision |
| `supersedes` | Replaces | NewDecision → OldDecision |
| `extracted_from` | Provenance | Fact → Session |
| `mentions` | References | Note → Person |
| `belongs_to` | Scoping | Convention → Workstream context |

### Entity Structure

```rust
struct Entity {
    id: Uuid,
    entity_type: EntityType,
    title: String,
    content: Option<String>,       // Markdown body
    confidence: f32,               // 0.0-1.0
    reinforcement_count: u32,      // How many times confirmed
    superseded: bool,
    tags: Vec<String>,
    source_session: Option<Uuid>,  // Which session created it
    created_at: DateTime,
    updated_at: DateTime,
    accessed_at: DateTime,
}
```

### Memory Store API

```rust
trait MemoryStore {
    // CRUD
    fn store_entity(&self, entity: &Entity) -> Result<()>;
    fn get_entity(&self, id: Uuid) -> Result<Option<Entity>>;
    fn update_entity(&self, entity: &Entity) -> Result<()>;
    fn delete_entity(&self, id: Uuid) -> Result<()>;

    // Relations
    fn add_relation(&self, source: Uuid, rel_type: RelationType, target: Uuid) -> Result<()>;
    fn get_relations(&self, entity_id: Uuid) -> Result<Vec<Relation>>;

    // Search
    fn search(&self, query: &str, limit: usize) -> Result<Vec<Entity>>;
    fn search_by_type(&self, entity_type: EntityType, limit: usize) -> Result<Vec<Entity>>;

    // Smart storage
    fn store_fact(&self, entity: &Entity) -> Result<StoreResult>;
    // → Inserted | Reinforced { existing } | Superseded { old }
    // Uses title + entity_type matching, search-before-create

    // Recall
    fn recall(&self, query: &str, context: &RecallContext) -> Result<Vec<RecallMatch>>;
    // Combines text search + relation traversal + confidence scoring
}
```

### Extraction Pipeline

**Trigger 1: On compaction** (T-0063)
When the engine compacts a session, extract key facts before context is lost:
1. Scan messages being summarized for extractable signals
2. For each signal: search KB, reinforce existing or create new
3. Link via `extracted_from` to session

**Trigger 2: Explicit** (`/remember` slash command)
User says "/remember the project uses PostgreSQL 15":
1. Parse the fact
2. Search-before-create in workstream KB
3. Store with `confidence: 1.0` (user-stated)

**Trigger 3: Hook-driven** (future)
Session start hook queries KB and injects relevant context. User prompt hook detects patterns ("we decided", "remember that", "the convention is").

### Session Injection

At session start, the engine loads:
1. **Global KB**: All `Preference` entities + high-confidence `Fact`/`Person` entities
2. **Workstream KB**: All `Convention`/`Decision` entities + recent `Fact`/`Note` entities
3. Inject as system prompt context (similar to how `arawn.md` context files work today)

### Agent Tools

Two core tools initially. `memory_relate` and richer graph traversal come later when the graph has enough data.

**`memory_store`** — Store knowledge with search-before-create

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `title` | string | yes | Concise title for the entity |
| `entity_type` | enum | yes | fact, decision, convention, preference, person, note |
| `content` | string | no | Markdown body with details |
| `tags` | string[] | no | Explicit categorization tags |
| `scope` | enum | no | global or workstream (inferred from entity_type if omitted: preference/person → global, decision/convention → workstream) |

**Output** (one of three):
- `Inserted` — new entity created, returns ID
- `Reinforced` — existing entity matched, incremented reinforcement count
- `Superseded` — conflicting entity found, old marked superseded, new created

Confidence is inferred: `Stated` (1.0) for explicit `/remember`, `Inferred` (0.5) for extraction on compaction.

**`memory_search`** — Multi-strategy retrieval across both tiers

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `query` | string | yes | Natural language search query |
| `entity_type` | enum | no | Filter by type |
| `tags` | string[] | no | Filter by tag intersection |
| `scope` | enum | no | global, workstream, or both (default: both) |
| `limit` | int | no | Max results (default: 10) |
| `include_related` | bool | no | Include graph-connected entities (default: false) |

**Retrieval stack** (results merged and ranked):
1. **Semantic search** — embedding similarity for conceptual matches
2. **FTS5** — fast exact/prefix text matching
3. **Tag filter** — cheap intersection when tags specified
4. **Graph expansion** — when `include_related`, follow relations 1 hop from matches

**Composite scoring**:
```
score = 0.4 * semantic_similarity + 0.3 * fts_score + 0.3 * confidence_score
```
Results deduplicated across strategies, highest score wins.

**Output**: Ranked list of entities with title, type, confidence score, reinforcement count, content snippet, tags, and optionally related entities with relation type.

### Embedding Configuration

Configurable embedding provider, same pattern as LLM backends. Defaults to local ONNX model (works offline, no API key needed).

```toml
# arawn.toml
[embeddings]
provider = "local"                    # default — ONNX runtime
# model = "all-MiniLM-L6-v2"         # default local model
# dimensions = 384

# API-based alternative:
# [embeddings]
# provider = "openai"
# model = "text-embedding-3-small"
# dimensions = 1536
# api_key_env = "OPENAI_API_KEY"
```

**Embedding trait**:
```rust
#[async_trait]
trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
    fn dimensions(&self) -> usize;
}
```

Implementations: `LocalEmbedder` (ONNX, ported from backup's vendored crates), `ApiEmbedder` (OpenAI/Anthropic embedding endpoints).

### Slash Commands

| Command | Description |
|---------|-------------|
| `/remember <text>` | Store an explicit fact/preference |
| `/memory` | Show KB summary (entity counts by type) |
| `/forget <id>` | Delete/supersede an entity |

## Detailed Design

### Port Strategy

The backup `arawn-memory` crate provides the foundation. Port in phases:
1. **Core types** — Entity, Relation, ContentType, Confidence from `types.rs`
2. **SQLite store** — entity CRUD, relations, FTS5 search from `store/`
3. **Recall** — text search + confidence scoring (skip vectors initially)
4. **Graph** — graphqlite integration for relation queries (or simpler adjacency table)

Simplifications vs backup:
- Skip vector embeddings initially (text search + graph traversal sufficient)
- Fewer entity types (6 vs backup's 9 content types)
- No citation system initially (track source_session instead)
- No ontology system (Clotho-specific, not needed for arawn's simpler model)

### Confidence Scoring

Ported from backup with simplified parameters:
```
score = base_score * reinforcement_boost * staleness_multiplier

base_score: Stated=1.0, Observed=0.7, Inferred=0.5
reinforcement_boost: min(1.0 + 0.1 * count, 1.5)
staleness: decays from 1.0 → 0.3 over 365 days
superseded entities: score = 0.0
```

### Search-Before-Create (Critical Pattern)

From Clotho: **never create without searching first**.

1. Agent wants to store "user prefers Rust"
2. Search workstream + global KB for "user prefer Rust"
3. If match found: reinforce existing entity (increment count, update accessed_at)
4. If contradiction found: supersede old, create new
5. If no match: create new entity

This preserves frequency as signal — an entity reinforced 5 times from different sessions is high-confidence knowledge.

## Testing Strategy

- **Unit tests**: Entity CRUD, relation operations, FTS5 search, confidence scoring
- **Integration tests**: Store-fact with search-before-create (insert/reinforce/supersede paths)
- **Integration tests**: Session injection (load KB into system prompt)
- **Integration tests**: Extraction on compaction (mock session → extracted entities)

## Alternatives Considered

- **Flat markdown files** (Claude Code approach): Simpler but can't link facts, no confidence scoring, no search-before-create dedup. Good for preferences, bad for accumulated project knowledge.
- **Full Clotho port**: Too many entity types and workflows for arawn's needs. Clotho is a standalone KB product; arawn needs a subsystem.
- **Vector-first approach** (backup's recall system): Requires embedding pipeline, adds latency and complexity. Graph + text search covers the main use cases without it.

## Implementation Plan

**Phase 1 — Core store**: Port entity types, SQLite store with CRUD + FTS5 search + relations. Two-tier (global + workstream) database initialization. Tags support. Unit tests.

**Phase 2 — Embedding infrastructure**: `Embedder` trait + `LocalEmbedder` (ONNX) + `ApiEmbedder`. Embedding config in arawn.toml. sqlite-vec integration for vector storage/search.

**Phase 3 — Memory tools**: `memory_store` and `memory_search` tools registered in engine. Search-before-create with composite scoring (semantic + FTS + confidence). Integration tests.

**Phase 4 — Session injection**: Load global + workstream KB at session start, inject into system prompt context. Configurable entity count limits.

**Phase 5 — Extraction on compaction**: When compactor runs, scan messages being summarized for extractable signals. Store as entities with `extracted_from` session link.

**Phase 6 — Slash commands**: `/remember`, `/memory`, `/forget` wired into the TUI command system (I-0014).

**Phase 7 — Session scratchpad**: `memory.md` in working dir, loaded/saved by engine, promoted to workstream KB on `/remember`.

**Future phases (not in scope):**
- `memory_relate` tool for explicit graph linking
- Hook-driven extraction (session start, prompt patterns)
- Entity consolidation agent (merge duplicates, archive stale)
- Graph visualization in TUI