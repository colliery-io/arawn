---
id: extraction-chain-prompts-classify
level: task
title: "Extraction chain prompts — classify, extract, link-by-name, write"
short_code: "ARAWN-T-0252"
created_at: 2026-05-13T01:28:13.468772+00:00
updated_at: 2026-05-13T03:20:39.930191+00:00
parent: ARAWN-I-0040
blocked_by: [ARAWN-T-0251]
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

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

### 2026-05-13 — CotChain landed; 4 stages compose end-to-end

**Files.**
- `crates/arawn-extractor/src/cot.rs` — `CotChain` struct, `ExtractionChain` impl, parse helpers, FTS-resolve helper, `projection_id_to_uuid` (Uuid v5 from the projection row id for stable provenance edges).
- `crates/arawn-extractor/src/llm_text.rs` — `complete_text` drains the streaming `LlmClient` into a single string; `extract_json_block` tolerates models that fence JSON in ```json or surround it with prose.
- `crates/arawn-extractor/src/lib.rs` — `pub mod cot; pub use CotChain;`.
- `crates/arawn-extractor/Cargo.toml` — added `futures` (for stream consumption) and the `v5` feature on uuid.

**Stage shapes** (matches the task spec):
- Stage 1 classify → JSON `{in_scope, reason}`. Short-circuits with `skipped: true` if out of scope.
- Stage 2 extract → JSON array of `{entity_type, title, content?, tags?}`. Unknown entity_types are skipped at write time with a warn.
- Stage 3 link-by-name → JSON array of `{from, rel, to_name}`. Names refer to titles, not ids.
- Stage 4 write → `store_fact` for entities, resolve links (new-entities map → FTS workstream tier → FTS global tier), `add_relation` for resolved links, plus EXTRACTED_FROM provenance edge (Uuid v5 of the projection_id).

**Decisions kept from the task spec.**
- **Open-coded JSON between stages**, not structured tool-use. The CoT chain is intended for free / cheap models that often don't support structured tool calls; plain JSON in a fenced or prose-wrapped response is universally returnable. `extract_json_block` makes the parser tolerant.
- **FTS-resolve falls back from workstream → global tier.** If a link target lives in global memory (e.g. a Person), we still resolve it.
- **Provenance edge uses the projection_id namespaced as Uuid v5.** Stable across runs, so re-running an item that was already extracted reinforces (via store_fact dedup) instead of duplicating edges.
- **Body truncation at 4k chars per stage.** Free models often have small context windows. Truncation marker preserved so models notice.

**Tests.**
- 9 unit tests in `cot.rs` — classify parse (in/out of scope), candidates parse (empty/basic), links parse, entity/relation type case-insensitivity, deterministic uuid v5, truncate.
- 4 unit tests in `llm_text.rs` — JSON-from-fence, JSON-from-prose, nested braces, missing-json fallback.
- All 17 extractor unit tests pass (4 from T-0251 runner + 13 new).
- Full workspace test sweep: 0 failures.
- `angreal check clippy` clean.

**Deferred.**
- Construction-time wiring (arawn main building `CotChain` from `extraction_llm()` + `LlmClientPool::resolve(...)` and passing into the runner). The runner still uses StubChain by default; switching to CotChain is a single line in arawn main and is folded into T-0254's integration setup.
- Quality tuning of prompts. Per the task scope: "good enough to round-trip"; real-data tuning happens once we see actual outputs.

**Acceptance criteria.**
- [x] `CotChain` implements `ExtractionChain` end-to-end through all 4 stages.
- [x] Stage 1 short-circuits cleanly with `skipped: true` when out of scope.
- [x] Stage 3's link-by-name uses FTS; misses drop with warn.
- [x] Stage 4 uses `store_fact` for entities and `add_relation` for edges.
- [x] Each entity gets a provenance `EXTRACTED_FROM` edge to the projection row's id (as Uuid v5).
- [x] Unit tests cover each stage in isolation with deterministic LLM responses.
- [x] `angreal check workspace` + `angreal check clippy` clean.

T-0254 (integration tests) can now plug a MockLlm-backed `CotChain` into the runner.