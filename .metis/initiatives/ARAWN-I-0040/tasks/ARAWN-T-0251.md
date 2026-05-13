---
id: extractor-plumbing-cursors
level: task
title: "Extractor plumbing — cursors, dispatch trigger, LLM config, chain trait"
short_code: "ARAWN-T-0251"
created_at: 2026-05-13T01:28:12.393919+00:00
updated_at: 2026-05-13T01:28:12.393919+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Extractor plumbing — cursors, dispatch trigger, LLM config, chain trait

## Parent Initiative

[[ARAWN-I-0040]]

## Objective

Stand up the per-workstream extractor plumbing: the cursor table that tracks which projection rows each workstream has processed, the dispatch-hook trigger that fires extraction after feed capture, the LLM-provider config (with fall-through to the global LLM), and the chain trait that T-0252 fills in with actual prompts.

This task ships an end-to-end skeleton: feed → projection → trigger → extractor → workstream KB. The chain's per-stage prompts are stubs; T-0252 replaces them with real prompts that emit typed entities + linked-by-name edges.

## Scope

### Cursor table

New migration `V5__extractor_cursors.sql`:

```sql
CREATE TABLE extractor_cursors (
    workstream_name  TEXT NOT NULL,
    feed_type        TEXT NOT NULL,           -- e.g. 'gmail_messages', 'slack_messages'
    last_source_ts   TEXT NOT NULL DEFAULT '', -- RFC3339 of highest source_ts processed
    last_processed_at TEXT NOT NULL,
    PRIMARY KEY (workstream_name, feed_type)
);
```

CRUD lives in `arawn-storage::ExtractorCursorStore`:
- `get(workstream, feed_type) -> Option<DateTime<Utc>>`
- `advance(workstream, feed_type, source_ts)` — monotonic
- `list_for_workstream(workstream)` — for `/workstream show` later

### Trigger from feeds dispatch

Reactive, not cron. After `arawn-feeds::dispatch::run_feed_inner` finishes the projection write step, fire a per-workstream extraction job. The trigger lives next to the existing `project_feed_dir` call. One job per (workstream, feed_type) that has new rows since the workstream's cursor.

For Phase 4 v1: run extractions synchronously inside the dispatch flow if the run is small (< 50 new rows), otherwise spawn a tokio task. Steady rhythm — small batches per feed cycle. Backfill on bind (T-0253) reuses the same execution path.

### LLM config with fall-through

`arawn.toml` gains an optional `[llm.extraction]` section. If present, the extractor uses that backend; if absent, it falls through to the global `[llm]` section. Same Provider/model/endpoint/api_key shape.

In code: `LlmClientPool::client_for(role)` where `role ∈ {Interaction, Extraction, …}`. Defaults to Interaction when role-specific config is absent.

### Chain trait

```rust
#[async_trait]
pub trait ExtractionChain: Send + Sync {
    async fn run(
        &self,
        workstream: &Workstream,
        row: &ProjectionRow,
        kb: &MemoryManager,
    ) -> Result<ChainOutcome, ExtractionError>;
}

pub struct ChainOutcome {
    pub entities_written: Vec<Uuid>,
    pub relations_written: usize,
    pub skipped: bool, // true if `classify` returned out-of-scope
}
```

T-0251 ships a stub impl (`StubChain`) that always returns `skipped: true`. T-0252 ships the real `CotChain` that does classify → extract → link-by-name → write.

### Runner

`ExtractorRunner` owns the cursor store + LLM pool + memory router + chain impl. Per-call API:

```rust
async fn run_for_workstream(
    &self,
    workstream: &Workstream,
    feed_type: &str,
) -> Result<RunStats, ExtractionError>
```

Looks up the cursor, queries `<feed_type> WHERE source_ts > cursor ORDER BY source_ts LIMIT BATCH`, iterates rows through the chain, advances the cursor on success.

### What's deferred

- Actual chain prompts (T-0252).
- Backfill spawn-loop for newly-bound sources (T-0253).
- Integration tests with mock LLM (T-0254).
- Tag vocabulary refinement — steward (Phase 5).
- Session-time "suggest new ontology terms" prompt — Phase 5/6.

## Acceptance Criteria

- [ ] `extractor_cursors` table created via V5 migration; CRUD round-trips.
- [ ] `ExtractionChain` trait + `StubChain` impl compile and round-trip a no-op.
- [ ] `ExtractorRunner` advances the cursor monotonically; idempotent on re-run.
- [ ] Feed dispatch hook fires the runner for each enabled workstream after projection writes. Soft-fails (warn-and-continue) per the existing T-0237 pattern.
- [ ] `[llm.extraction]` config parses; falls through to `[llm]` when absent.
- [ ] Unit tests cover: cursor CRUD, runner-with-StubChain advances cursor, fall-through llm config resolution.
- [ ] `angreal check workspace` + `angreal check clippy` clean.

## Implementation Notes

### Crate placement

New crate `arawn-extractor`. Sits between `arawn-projections` (reads from), `arawn-memory` (writes to), `arawn-llm` (model calls), and `arawn-feeds` (trigger point).

### Workstream enumeration on trigger

At trigger time the runner needs the list of active workstreams. Queries `WorkstreamStore::list()` (active only — archived skipped) and dispatches per workstream. The runner doesn't filter by bindings — per the design discussion, each workstream's extractor sees every new projection row and decides via the classify stage.

### Dependencies

- T-0240 (projections exist with `<feed_type>` table + source_ts).
- T-0248 (WorkstreamStore + Workstream record).
- `arawn-feeds::dispatch::run_feed_inner` (trigger point).
- `arawn-llm` (LlmClientPool with role-based selection).

### Risk considerations

- **Long extraction blocking feeds.** A workstream's extraction can take seconds per row. With 15 workstreams × 50 rows that's minutes. Mitigation: spawn into a tokio task when batch > 50; the dispatch hook fires-and-forgets.
- **LLM rate limits.** Free/cheap backends have lower limits than the main interaction model. The pool has the standard retry-with-backoff path; extractor inherits it.
- **Cursor drift on errors.** If extraction fails for row N, the cursor stays at N-1 so the next run retries from N. A row that consistently fails will block the workstream's progress forever — surface to the steward in Phase 5; for v1 we accept this and log loud.

## Status Updates

*To be added during implementation*