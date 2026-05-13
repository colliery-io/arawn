---
id: extractor-plumbing-cursors
level: task
title: "Extractor plumbing — cursors, dispatch trigger, LLM config, chain trait"
short_code: "ARAWN-T-0251"
created_at: 2026-05-13T01:28:12.393919+00:00
updated_at: 2026-05-13T03:15:41.856736+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

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

### 2026-05-13 — Skeleton landed end-to-end with StubChain

**Files.**
- `crates/arawn-storage/migrations/V5__extractor_cursors.sql` — new table.
- `crates/arawn-storage/src/extractor_cursor_store.rs` — `ExtractorCursorStore` with `get / advance (monotonic) / list_for_workstream`. 4 unit tests.
- `crates/arawn-storage/src/store.rs` — `pub fn database()` accessor so the extractor can build its own per-table store on the shared `Database`.
- `crates/arawn-projections/src/store.rs` — `conn()` accessor promoted from `pub(crate)` to `pub` so the extractor can run paged queries against `<feed_type>` tables.
- `crates/arawn-extractor/` (new crate) — `chain.rs` (trait + `ChainOutcome` + `StubChain`), `runner.rs` (`ExtractorRunner` with `run_for_workstream` + `run_for_all_workstreams` + `MemoryResolver` closure type + projection paging), `error.rs`, `lib.rs`. 4 integration tests.
- `crates/arawn-feeds/src/dispatch.rs` — `FeedRuntimeContext.extractor: Option<Arc<ExtractorRunner>>`; `run_feed_inner` fans out to `extractor.run_for_all_workstreams(feed_type)` for each projection feed_type the template touched, after projection writes. `projection_feed_types_for(template_name)` helper maps `gmail/*` → `gmail_messages`, `slack/*` → both slack tables, etc.
- `crates/arawn-feeds/src/runtime.rs` — `start()` takes a seventh arg `extractor: Option<Arc<ExtractorRunner>>`.
- `crates/arawn-feeds/tests/*.rs` — all 9 `arawn_feeds::start` call sites updated with the new arg.
- `crates/arawn/src/config.rs` — `ExtractionConfig { llm: Option<String> }` added to `ArawnConfig`. New methods: `extraction_llm()` + `extraction_llm_name()`, both falling through to engine when absent.
- `crates/arawn/src/main.rs` — workstream router hoisted to outer scope; built once and shared between memory tools and the extractor's `MemoryResolver` closure. Extractor wired into `arawn_feeds::start` when both projections and the router are available. StubChain for now — T-0252 swaps in `CotChain`.

**Routing flow (steady state).** Feed dispatch finishes template run → projection writes happen → for each projection feed_type the template wrote (e.g. `slack_messages` + `slack_thread_messages` for slack archives), `ExtractorRunner::run_for_all_workstreams(feed_type)` iterates every active workstream. Each workstream: read cursor → query `<feed_type> WHERE source_ts > cursor ORDER BY source_ts LIMIT 50` → run the chain (StubChain returns skipped) → advance cursor to highest processed source_ts. Errors block cursor advancement so retry happens on next cycle.

**Decisions worth keeping.**
- **Cursor monotonicity at the SQL layer.** The `advance` SQL uses a CASE in the ON CONFLICT clause so a backwards advance is silently a no-op. Defense against concurrent writers (e.g. backfill running while dispatch fires) clobbering each other.
- **MemoryResolver as closure.** Lets the extractor depend only on `arawn_memory::MemoryManager` while in production the closure points at the same `WorkstreamMemoryRouter` cache the memory tools use. Test code passes a closure that opens an in-memory `MemoryManager` per call. No new trait.
- **No direct `arawn-llm` use in T-0251.** StubChain doesn't call the LLM, so the actual `LlmClientPool` plumbing is just the config + `extraction_llm()` accessor. T-0252 will pick that up when writing the real chain.
- **Reactive trigger, not cron.** Already discussed in the decomposition; reaffirmed in the implementation. Dispatch hook fires after projection writes; backfill (T-0253) will reuse the same `run_for_workstream` entry point.

**Tests.**
- 4 unit tests in `arawn_storage::extractor_cursor_store::tests` (get-none-for-unknown, insert-then-update, no-backwards, list-for-workstream).
- 4 integration tests in `arawn_extractor::runner::tests` (empty-table no-op, StubChain advances cursor + marks skipped, re-run is no-op, run-for-all-workstreams skips archived).
- Full workspace test sweep: 0 failures across all crates.
- `angreal check clippy` clean.

**Acceptance criteria.**
- [x] `extractor_cursors` table created via V5 migration; CRUD round-trips.
- [x] `ExtractionChain` trait + `StubChain` impl compile and round-trip a no-op.
- [x] `ExtractorRunner` advances the cursor monotonically; idempotent on re-run.
- [x] Feed dispatch hook fires the runner for each enabled workstream after projection writes. Soft-fails per existing T-0237 pattern.
- [x] `[llm.extraction]` config parses; falls through to `[llm]` when absent.
- [x] Unit tests cover the named surfaces.
- [x] `angreal check workspace` + `angreal check clippy` clean.

T-0252 (CotChain) and T-0253 (backfill) unblocked.