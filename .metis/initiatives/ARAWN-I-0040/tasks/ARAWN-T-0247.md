---
id: feed-search-agent-tool-cross-feed
level: task
title: "feed_search agent tool — cross-feed semantic + structured search"
short_code: "ARAWN-T-0247"
created_at: 2026-05-12T03:28:20.350220+00:00
updated_at: 2026-05-12T03:28:20.350220+00:00
parent: ARAWN-I-0040
blocked_by: [ARAWN-T-0242]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# feed_search agent tool — cross-feed semantic + structured search

## Parent Initiative

[[ARAWN-I-0040]]

## Objective

Expose the projection layer to agents via a `feed_search` tool: filter by feed type, time range, structured fields; rank by FTS + semantic similarity (RRF fusion). This is the no-workstream fallback search and the first agent-facing surface produced by Phase 2.

## Scope

### Tool surface

Register a `feed_search` tool in `arawn-tool` / `arawn-engine`:

```
feed_search(
  query: string,                    // free-text; goes to FTS + embedding
  feed_types: [string]?,            // restrict to one or more projection tables
  since: string?,                   // RFC3339; filter on source_ts
  until: string?,                   // RFC3339
  filters: {field: value, ...}?,    // structured equality filters (sender, channel, project_key, …)
  limit: u32?                       // default 10, max 50
) -> [
  {
    feed_type: string,
    id: string,
    source_ts: string,
    title: string,                  // synthesized per feed type (e.g. "Re: foo from alice@", "ENG-123: Migrate auth")
    snippet: string,                // FTS-highlighted body excerpt
    score: f32,                     // RRF fused score
    metadata: {...}                 // feed-type-specific fields
  }
]
```

### Search pipeline

1. Per requested feed type:
   - FTS5 MATCH against the projection's FTS table → ranked ids.
   - Embed the query, search the projection's vector table → ranked ids.
   - Apply structured filters + time range at the projection table level.
   - RRF-fuse the two ranked lists (reuse the helper from `longmemeval_bench.rs::reciprocal_rank_fusion`).
2. Merge per-feed-type results; sort by fused score; truncate to `limit`.
3. Hydrate each row: fetch from projection table for the snippet + metadata.

### Documentation

New page `docs/src/feeds/feed-search.md` covering the tool surface + 5-7 worked agent prompts.

### What's deferred

- Per-workstream `signal_search` — Phase 6.
- Cross-projection JOINs (e.g. "the jira issue mentioned in this slack message") — Phase 4 or later.

## Acceptance Criteria

- [ ] `feed_search` tool registered in the engine and discoverable to agents.
- [ ] Supports `feed_types` filter, time-range filter, structured-field filter.
- [ ] Hybrid FTS + vector + RRF fusion; results consistently ranked.
- [ ] `docs/src/feeds/feed-search.md` exists with tool spec + worked prompts.
- [ ] Unit tests exercise filter combinations; integration test calls the tool end-to-end against fixture projections.
- [ ] `angreal check workspace` + `angreal check clippy` clean.

## Implementation Notes

- Reuse arawn-embed for query embedding.
- The RRF helper in `longmemeval_bench.rs` is a starting point — extract it into a shared util if not already there.
- Time-range filters need indexes on `source_ts` per projection table (Calendar's `start_ts` is the exception — index on that too).
- Tool description should be opinionated about *when* to use it (cross-feed lookups, no workstream) vs the future `signal_search` (workstream-scoped).

### Dependencies

- T-0242 (plumbing + gmail reference).
- At least one of T-0243 / T-0244 / T-0245 / T-0246 — but ideally all four, so the tool covers all 9 feed types.

## Status Updates

*To be added during implementation*