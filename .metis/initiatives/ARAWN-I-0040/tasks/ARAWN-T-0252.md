---
id: extraction-chain-prompts-classify
level: task
title: "Extraction chain prompts — classify, extract, link-by-name, write"
short_code: "ARAWN-T-0252"
created_at: 2026-05-13T01:28:13.468772+00:00
updated_at: 2026-05-13T01:28:13.468772+00:00
parent: ARAWN-I-0040
blocked_by: [ARAWN-T-0251]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Extraction chain prompts — classify, extract, link-by-name, write

## Parent Initiative

[[ARAWN-I-0040]]

## Objective

Replace the `StubChain` from T-0251 with `CotChain` — the actual 4-stage CoT extraction chain that turns a projection row into typed entities + linked relations in the workstream's KB. Free / inexpensive model behind it; per-stage prompts are small enough that running per-workstream per-row stays affordable.

The chain reads the workstream's description (set via `/workstream describe`) to decide scope. Tags emit freely; the steward refines vocabulary in Phase 5.

## Scope

### Stage 1: classify

**Input**: workstream name + description + projection row (title + body_text + feed_type).
**Output**: `{ in_scope: bool, reason: string }`.

Prompt instructs the model to read the workstream description carefully and answer "would this projection row produce a fact / decision / note worth keeping in this workstream's KB?". `reason` is logged for debugging; it's not stored.

Short-circuits the rest of the chain when `in_scope = false`.

### Stage 2: extract

**Input**: workstream description + projection row.
**Output**: a list of candidate entities:

```json
[
  {
    "entity_type": "decision" | "fact" | "convention" | "preference" | "person" | "note",
    "title": "...",
    "content": "...",
    "tags": ["..."]
  }
]
```

The closed `EntityType` enum is enforced by Rust on the way in (invalid types → skipped with warn). Tags are free-form ASCII slugs; the steward stabilizes them later.

### Stage 3: link-by-name

**Input**: extracted entities (Stage 2 output) + workstream description.
**Output**: a list of proposed relations referencing existing entities by their **title**, not id:

```json
[
  {
    "from": "<title of one of the new entities>",
    "rel": "relates_to" | "supports" | "contradicts" | "supersedes" | "mentions" | "belongs_to",
    "to_name": "<title of an existing OR new entity>"
  }
]
```

The model can reference both newly-extracted entities and existing entities by title. After the LLM returns:

1. Build a name → id map for the new entities (from Stage 2).
2. For each proposed link whose `to_name` isn't in the new map, FTS-search the workstream's KB (`MemoryStore::search_by_type` or `search`) for an entity with that title. If a strong match (top-1 with score > threshold) is found, resolve to its id. Otherwise drop the link with a warn.
3. Build the `Relation` list with resolved Uuids.

This is the **link-by-name** pattern we settled on — keeps prompts short, doesn't grow with KB size, and tolerates typos via FTS's fuzziness.

### Stage 4: write

**Input**: resolved entities + resolved relations.
**Output**: `ChainOutcome { entities_written, relations_written, skipped: false }`.

1. For each entity: `store.store_fact(&entity)` (handles dedup via search-before-create). Records the returned `Uuid` keyed by `title` for relation resolution.
2. For each resolved relation: `store.add_relation(from_uuid, rel, to_uuid)`. Skip if either side is None.
3. Add an `EXTRACTED_FROM` edge from each new entity to the projection row's `id` (provenance).

### Provider config

Reads `LlmClientPool::client_for(Extraction)` — falls through to Interaction when not configured. Same retry-with-backoff path that the rest of arawn-llm uses.

### What's deferred

- Backfill (T-0253).
- Integration tests with mock LLM (T-0254).
- Tag vocabulary refinement — steward (Phase 5).
- Quality tuning of individual prompts — separate follow-up once we have real data.

## Acceptance Criteria

- [ ] `CotChain` implements `ExtractionChain` (from T-0251) end-to-end through all 4 stages.
- [ ] Stage 1 short-circuits cleanly (returns `skipped: true`) when out of scope.
- [ ] Stage 3's link-by-name resolution uses FTS against the active workstream's KB; misses are dropped with a warn.
- [ ] Stage 4 uses `store_fact` (dedup) for entities and `add_relation` for edges.
- [ ] Each entity gets a provenance `EXTRACTED_FROM` edge to the projection row's id.
- [ ] Unit tests cover each stage in isolation with deterministic LLM responses.
- [ ] `angreal check workspace` + `angreal check clippy` clean.

## Implementation Notes

### Prompt engineering scope

The first version of each prompt is "good enough to round-trip" — short, clear, robust to model variation. Per-prompt quality tuning is a separate follow-up once we see real outputs from real workstreams. Don't gold-plate.

### LLM response parsing

Each stage expects a JSON response. Use serde to parse; on malformed JSON, log loud and drop the row (cursor stays put so we retry once but a consistently-broken row blocks progress — steward fixes that in Phase 5).

### Dependencies

- T-0251 (the chain trait, runner, cursor table, LLM pool role wiring).

## Status Updates

*To be added during implementation*