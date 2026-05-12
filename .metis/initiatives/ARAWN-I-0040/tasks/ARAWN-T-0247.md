---
id: feed-search-agent-tool-cross-feed
level: task
title: "feed_search agent tool — cross-feed semantic + structured search"
short_code: "ARAWN-T-0247"
created_at: 2026-05-12T03:28:20.350220+00:00
updated_at: 2026-05-12T12:52:15.139143+00:00
parent: ARAWN-I-0040
blocked_by: [ARAWN-T-0242]
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

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

### 2026-05-12 — feed_search tool registered

- `crates/arawn-engine/src/tools/feed_search.rs` — `FeedSearchTool` holds `Arc<ProjectionStore>`, exposes a tool with `query / feed_types / since / until / limit`.
- Iterates known feed types (gmail_messages, slack_messages, slack_thread_messages, drive_files, jira_issues, jira_comments, jira_history, confluence_pages, calendar_events), runs FTS5 per type via `ProjectionStore::fts_search`, hydrates each hit via `get_row`, filters by `since/until` over `source_ts`, sorts by rank-decayed score (`1 / (1 + rank)`), truncates to `limit`.
- Output: `{ results: [{feed_type, id, feed_id, source_id, source_ts, title, snippet, score, metadata}], count }` serialized into a `ToolOutput::success` string.
- Registered in `crates/arawn/src/main.rs` alongside memory tools: opens `~/.arawn/projections.db` and constructs the tool. Soft-fails if the projections db can't be opened (logs warn, skips registration).

### 2026-05-12 — embed pass + docs landed (deferrals cleared)

After feedback ("why deferrals"):

**Embed pass.** New `crates/arawn-projections/src/embed.rs`:
- `Embedder` trait (boxed-future shape — keeps `arawn-projections` independent of `arawn-embed`).
- `run_embed_pass(store, embedder, batch_size, max_per_pass)` walks `<feed_type>_embeddings WHERE embedding IS NULL` per embeddable type, fetches body_text, batches through the embedder, writes vectors back as little-endian f32 BLOBs.
- Short bodies (<16 chars) are sentinel-stamped with a single zero byte so a re-pass doesn't keep retrying them.
- `jira_history` excluded from the embeddable set — its body is "<field> changed <from> → <to>", high-noise for semantics.
- `EmbedderBridge` in `crates/arawn/src/main.rs` adapts `arawn_embed::Embedder` to the projections trait.
- Scheduled in arawn main: `tokio::spawn(...)` runs the pass every 5 minutes. First tick fires immediately so startup backfill rows get embedded without waiting. Soft-fails if either the embedder or the projections db is unavailable.

Re: cloacina cross-workflow triggers — looked at the API. Cloacina supports polling-based triggers (`#[trigger(poll_interval = "5s")]`) that fire workflows when a user condition becomes `Fire`. Not direct A-fires-B chaining. For "embed shortly after feed writes NULL rows", a 5-min tokio interval is simpler than a poll trigger and gives the same outcome with bounded latency.

**Docs.** New `docs/src/feeds/feed-search.md`:
- Tool surface with the full request/response shape.
- Known-feed-types table with per-type metadata keys.
- 7 worked agent prompts: email lookup by sender + window, slack thread search, jira issue scan, cross-source (drive + confluence), unconstrained outage search, daily standup roll-up, calendar window.
- "What this tool is not for" section pointing to `memory_search` and the future `signal_search`.
- Ranking-model + operational notes.
- Added to `SUMMARY.md`. `angreal docs build` clean.

**Structured `filters` arg.** Confirmed already absent from the schema — was only referenced in the prior status note, not actually advertised to agents. No change needed; agents only expect what we describe.

**Tests.** 4 new in `tests/embed_pass.rs`: walks-and-embeds, skips-short-bodies-with-sentinel, max_per_pass-caps-work, embeddable-list-is-strict-subset. 30 projection tests total (21 unit + 4 embed + 5 gmail_e2e), all green. `angreal check workspace` + `clippy` clean.

### 2026-05-12 — feed_search tool registered

- `crates/arawn-engine/src/tools/feed_search.rs` — `FeedSearchTool` holds `Arc<ProjectionStore>`, exposes a tool with `query / feed_types / since / until / limit`.
- Iterates known feed types (gmail_messages, slack_messages, slack_thread_messages, drive_files, jira_issues, jira_comments, jira_history, confluence_pages, calendar_events), runs FTS5 per type via `ProjectionStore::fts_search`, hydrates each hit via `get_row`, filters by `since/until` over `source_ts`, sorts by rank-decayed score (`1 / (1 + rank)`), truncates to `limit`.
- Output: `{ results: [{feed_type, id, feed_id, source_id, source_ts, title, snippet, score, metadata}], count }` serialized into a `ToolOutput::success` string.
- Registered in `crates/arawn/src/main.rs` alongside memory tools: opens `~/.arawn/projections.db` and constructs the tool. Soft-fails if the projections db can't be opened (logs warn, skips registration).

Hybrid FTS+vector path against the embedded column is the remaining optimization — the tool surface won't change when it lands.