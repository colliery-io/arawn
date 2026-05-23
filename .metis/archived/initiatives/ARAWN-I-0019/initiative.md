---
id: layered-memory-retrieval-budgeted
level: initiative
title: "Layered memory retrieval — budgeted context injection with entity compression"
short_code: "ARAWN-I-0019"
created_at: 2026-04-07T21:17:02.737698+00:00
updated_at: 2026-04-09T16:42:40.812500+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: layered-memory-retrieval-budgeted
---

# Layered memory retrieval — budgeted context injection with entity compression Initiative

## Context

The current memory injection pipeline in `local_service.rs` loads all matching KB entities and dumps them into the system prompt without budgeting. This doesn't scale — as the KB grows, the injected context competes with the conversation for token space. There's no prioritization, no layering, and no compression.

Inspired by mempalace's 4-layer memory stack, this initiative restructures how memories are injected into the prompt:
- Always-on layers (identity + essential facts) stay within a fixed token budget
- On-demand retrieval loads topic-specific context when needed
- Entity shortcodes compress repeated names in the compact layers

### What exists today

- `arawn-memory` crate: SQLite + FTS5 store with entities, relations, confidence scoring, vector similarity
- Two-tier scoping: global KB + workstream-scoped KB
- `load_memories_for_injection()` in `inject.rs`: loads entities, formats as text, returns a Vec<String>
- `local_service.rs:send_message()`: injects memories into `PromptContext.memories`
- `system_prompt.rs`: renders memories into the system prompt as a flat block
- Agent tools: `memory_search` (vector + FTS), `memory_store` (CRUD)

### Problem

1. **No budget**: All matching entities injected regardless of token cost
2. **No priority**: A stale inferred fact gets the same prompt space as a fresh user-stated preference
3. **No layering**: Can't distinguish "always loaded" from "load on demand"
4. **No compression**: Entity names repeated verbatim, wasting tokens in the compact layers

## Goals & Non-Goals

**Goals:**
- Replace flat memory injection with a 4-layer stack: L0 (identity), L1 (essential), L2 (on-demand), L3 (deep search)
- Budget L0+L1 to ~900 tokens — always injected, never exceeds budget
- L1 auto-ranks entities by confidence score, recency, and type
- Entity shortcodes in L1 output for token compression (store raw, compress on render)
- L2 available as an automatic or agent-callable retrieval filtered by topic/tags
- Existing `memory_search` tool becomes L3 (no change needed)

**Non-Goals:**
- AAAK as a storage format — we store raw, compress only in the rendered output
- Emotional encoding — irrelevant for a dev tool context
- Conversation mining / batch ingestion — separate concern
- Temporal knowledge graph — future initiative

## Detailed Design

### Layer 0 — Identity (~100 tokens, always loaded)

Static text generated from workstream metadata:
- Workstream name and purpose
- Key people/entities mentioned in Person-type entities
- Core conventions (Convention-type entities with highest confidence)

Source: workstream-scoped entities of type Person and Convention, plus workstream metadata.

### Layer 1 — Essential Story (~500-800 tokens, always loaded)

Auto-generated on each `send_message` call from the top-ranked entities:
1. Query both global and workstream KB
2. Score entities by `confidence_score()` (accounts for source, reinforcement, staleness)
3. Take top N entities that fit within ~800 token budget
4. Group by entity_type for readability
5. Apply entity shortcodes: build a legend of name→code mappings for entities that appear 2+ times, substitute in the output

Format:
```
[L1 — KEY FACTS] (AE=arawn-engine, RS=Rust)
[decisions] Switched to layered prompt injection for memory budget control
[conventions] Tests inline, not separate files. Never pipe cargo through grep.
[preferences] User prefers simplest solution first.
[facts] AE uses tracing for logging. TUI connects to server via WebSocket.
```

### Layer 2 — On-Demand (~200-500 tokens per retrieval)

Topic-filtered retrieval loaded when a specific subject comes up. Two triggers:
1. **Automatic**: When the user's message mentions entities that match KB tags, auto-inject relevant L2 context before the LLM call
2. **Agent-initiated**: The existing `memory_search` tool already serves this purpose

Implementation: Add a `retrieve_topical(tags: &[String], budget: usize)` method to MemoryManager that does filtered search with a token budget.

### Layer 3 — Deep Search (unlimited)

Already exists: `memory_search` tool does FTS5 + optional vector similarity search. No changes needed.

### Entity Shortcodes

Applied only to L0 and L1 rendered output, not to storage:
1. Scan L1 text for entity names that appear 2+ times
2. Generate 2-3 char codes (first letters, or configured aliases)
3. Prepend a legend line: `(AE=arawn-engine, RS=Rust, DS=Dylan Storey)`
4. Substitute in the body text

This is simple string replacement, not a dialect. Saves ~15-30% tokens in L1 when entities are repeated.

### Integration Point

In `local_service.rs:send_message()`, replace:
```rust
let kb_memories = arawn_memory::load_memories_for_injection(mgr, None, None);
ctx.memories = kb_memories;
```
With:
```rust
let memory_stack = arawn_memory::MemoryStack::new(mgr);
let l0_l1 = memory_stack.wake_up(900); // budget in tokens
ctx.memories = vec![l0_l1];
// L2 auto-injection happens inside the engine before the LLM call
```

## Alternatives Considered

- **Adopt mempalace wholesale**: Wrong tool — it's Python/ChromaDB, we're Rust/SQLite. The architecture ideas transfer; the implementation doesn't.
- **AAAK as storage format**: Their own benchmarks show 12.4% retrieval regression vs raw. Store raw, compress only on render.
- **ChromaDB for vector search**: We already have optional embedding support in arawn-memory. SQLite + FTS5 is simpler and has no external dependencies.
- **No budgeting, just better ranking**: Doesn't solve the fundamental problem — even well-ranked entities will eventually exceed context budget.

## Technical Reference

### Current injection pipeline (to be replaced)

**Entry point**: `crates/arawn/src/main.rs:334-343`
```rust
if let Some(ref mgr) = memory_manager {
    let kb_memories = arawn_memory::load_memories_for_injection(mgr, None, None);
    if !kb_memories.is_empty() {
        if let Some(ref mut ctx) = engine_config.prompt_context {
            ctx.memories = kb_memories;
        }
    }
}
```
This runs once at server startup. Memories are baked into the engine config and reused for every message. They're NOT refreshed per-session or per-message.

**Per-session override**: `crates/arawn/src/local_service.rs:609-625` — builds a per-session `PromptContext` but just clones `pc.memories` from the startup config (line 622). So even in the per-session path, the memories are stale from boot time.

**Injection function**: `crates/arawn-memory/src/inject.rs:16-92` — `load_memories_for_injection(mgr, global_limit, workstream_limit)`. Queries both tiers, groups by type, formats as markdown strings. Hard limit of 20 global + 30 workstream entities. No token budgeting — just entity count limits.

**Rendering**: `crates/arawn-engine/src/system_prompt.rs:247-263` — `SystemPromptBuilder::memories()` takes `&[String]`, renders as a `# Relevant Memories` section with priority 6. Each string becomes a `- {memory}\n` bullet.

### Key types and methods needed

**MemoryManager** (`crates/arawn-memory/src/manager.rs`):
- `pub global: Arc<MemoryStore>` — global KB
- `pub workstream: Arc<MemoryStore>` — workstream-scoped KB
- `store_for_type(et: EntityType) -> &Arc<MemoryStore>` — routes by default scope

**MemoryStore** (`crates/arawn-memory/src/store.rs`):
- `list_by_type(et, limit) -> Vec<Entity>` — ordered by `updated_at DESC`
- `search_similar(embedding, limit) -> Vec<SimilarityResult>` — vector search
- `search_similar_filtered(embedding, entity_type, limit)` — vector + type filter
- No method to list ALL entities ranked by confidence — **needs to be added**

**Entity** (`crates/arawn-memory/src/types.rs`):
- `confidence_score() -> f32` — computed from `confidence_source`, `reinforcement_count`, days since update, `superseded`
- `entity_type: EntityType` — Fact, Decision, Convention, Preference, Person, Note
- `title: String`, `content: Option<String>`, `tags: Vec<String>`

**Token estimation**: `crates/arawn-engine/src/token_estimator.rs` — `TokenEstimator::estimate_text(text) -> usize`. Uses `len / 4` heuristic. Good enough for budgeting.

### New code locations

- `crates/arawn-memory/src/stack.rs` — new file. `MemoryStack` struct with `wake_up(budget) -> String` method. Owns reference to `MemoryManager`.
- `crates/arawn-memory/src/inject.rs` — keep existing function for backwards compat, add `load_layered(mgr, budget) -> String` that uses the stack.
- `crates/arawn-memory/src/store.rs` — add `list_all_ranked(limit) -> Vec<Entity>` that queries all non-superseded entities ordered by confidence score.
- `crates/arawn-memory/src/shortcodes.rs` — new file. `apply_shortcodes(text, min_occurrences) -> String` scans for repeated entity names and substitutes.
- `crates/arawn/src/main.rs` — change injection site to use `MemoryStack::wake_up()`.
- `crates/arawn/src/local_service.rs:send_message()` — inject fresh L0+L1 per message instead of reusing startup config.

### MemoryStore query needed for ranked retrieval

Current `list_by_type` orders by `updated_at DESC`. For L1 we need all types ranked by confidence. New query:
```sql
SELECT * FROM entities
WHERE superseded = 0
ORDER BY
  CASE confidence_source
    WHEN 'stated' THEN 3
    WHEN 'observed' THEN 2
    WHEN 'inferred' THEN 1
  END DESC,
  reinforcement_count DESC,
  updated_at DESC
LIMIT ?
```
This gives stated > observed > inferred, with ties broken by reinforcement count, then recency.

### Token budget accounting

Use `text.len() / 4` (same heuristic as TokenEstimator). Build L0 first, measure tokens, allocate remaining to L1. Algorithm:
```
budget = 900 tokens
l0 = render_l0(mgr)  // ~100 tokens
remaining = budget - estimate_tokens(l0)
l1 = render_l1(mgr, remaining)  // fills to budget
return l0 + "\n" + l1
```

### Shortcode algorithm

1. Build the full L1 text without shortcodes
2. Count occurrences of each entity title (case-insensitive)
3. For entities appearing 2+ times: generate code from first letters of each word (e.g., "arawn-engine" → "AE", "Dylan Storey" → "DS")
4. Prepend legend: `(AE=arawn-engine, DS=Dylan Storey)`
5. Replace all occurrences in the body
6. Re-measure tokens — shortcodes should reduce by ~15-30%

### L2 auto-injection (Phase 3)

When `send_message` receives user text:
1. Extract keywords/entity names from the user message
2. Search KB by tags matching those keywords
3. If matches found that aren't already in L1, inject as a `[L2 — CONTEXT]` section
4. Budget L2 to ~400 tokens

This happens in `local_service.rs:send_message()` AFTER building the per-session PromptContext but BEFORE calling `engine.run()`. The user message content is available at that point (line 571 in current code).

## Implementation Plan

**Phase 1**: MemoryStack + L0/L1 generation with token budgeting
**Phase 2**: Entity shortcode compression for L1 output
**Phase 3**: L2 auto-injection (topic-triggered context loading)
**Phase 4**: Integration test + tuning (budget sizes, ranking weights)