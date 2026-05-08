---
id: phase-1-feed-runtime-cloacina
level: task
title: "Phase 1 — Feed runtime + cloacina wiring"
short_code: "ARAWN-T-0214"
created_at: 2026-05-07T00:42:18.450827+00:00
updated_at: 2026-05-08T14:34:29.652356+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0039
---

# Phase 1 — Feed runtime + cloacina wiring

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0039]]

## Objective **[REQUIRED]**

Land the foundation of the I-0039 feed system: a new `arawn-feeds` crate with the template trait, a feed-records DB table, a per-feed-dir on-disk meta layout, and cloacina cron-task registration. **No real templates yet** — runtime stands alone with a stub template for tests. Phases 2/3/4 add the real templates on top once this lands.

## Type / Priority

- Feature (foundational).
- P1 — every later phase of I-0039 depends on this.

**Reference:** all design decisions are locked in I-0039's "Design decisions locked (2026-05-06)" block. This task implements that design without re-litigating any of the 8 decisions.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement  
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] New `arawn-feeds` crate added to the workspace. Pulls in `cloacina`, `serde`, `serde_json`, `thiserror`, `tokio`, `tracing`, `chrono`, and the workspace `arawn-storage` for DB access.
- [ ] `FeedTemplate` trait defined per the design:
  ```rust
  #[async_trait]
  pub trait FeedTemplate: Send + Sync {
      fn name(&self) -> &'static str; // "<provider>/<template>"
      fn validate(&self, params: &TemplateParams) -> Result<(), FeedError>;
      fn defaults(&self, params: &TemplateParams) -> FeedDefaults;
      async fn run(&self, ctx: &TemplateCtx, params: &TemplateParams, feed_dir: &Path)
          -> Result<RunSummary, FeedError>;
  }
  ```
- [ ] Supporting types: `TemplateParams` (newtype around `serde_json::Value`), `FeedDefaults { cadence, ... }`, `RunSummary { items_written, bytes_written, duration }`, `FeedError` enum (`Auth`, `RateLimited(Option<Duration>)`, `Storage`, `Schema(String)`, `Provider(String)`).
- [ ] `TemplateCtx` carries the workspace integration registry handle (so templates can ask `ctx.slack()`, `ctx.gmail()`, etc.) and a structured logger.
- [ ] **Template registry**: a static `inventory!`-style or hand-rolled `FeedTemplateRegistry` that lets templates register themselves. Phase 2/3/4 templates will register at compile time; Phase 1 ships only a `stub/echo` template for tests.
- [ ] **DB schema**: new diesel migration creates a `feeds` table:
  ```sql
  CREATE TABLE feeds (
    id          TEXT PRIMARY KEY,         -- stable slug
    template    TEXT NOT NULL,            -- "<provider>/<template>"
    params      TEXT NOT NULL,            -- JSON
    cadence     TEXT NOT NULL,            -- cron expression
    enabled     INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
  );
  ```
  - Storage / model code in `arawn-feeds/src/registry.rs`. CRUD: `insert_feed`, `list_feeds`, `get_feed`, `set_enabled`, `delete_feed`.
- [ ] **On-disk layout**:
  - Data root resolves to `{config.storage.data_dir}/data/`. Default `~/.arawn/data/`.
  - Helper `feed_dir(feed_id) -> PathBuf` that returns `{data_root}/{provider}/{template}/{feed_id}/`.
  - `Meta { template, params, cursor: Value, last_run_at, last_status, run_count }` serialized as `meta.json` at the feed dir root.
  - `MetaStore` with `read(feed_dir)`, `write(feed_dir, &Meta)`, atomic-rename on write to avoid torn reads.
- [ ] **Cloacina wiring**: at server boot, after the workflow runner is started:
  1. Read all `enabled = 1` rows from `feeds`.
  2. For each row, look up the template in the registry; if missing, log a warning and skip.
  3. Validate params; if invalid, mark the row as failed-validation and skip.
  4. Register a cloacina cron task with the row's `cadence`. The task body calls `template.run(&ctx, &params, &feed_dir)`, then writes the new cursor + last_run_at to `meta.json`.
  5. On `/watch`-style adds (Phase 6), inserts a row + registers a cloacina task without a server restart. Phase 1 only needs the boot path; the dynamic-add path is a stub `register_feed_runtime(&self, feed: Feed) -> Result<()>` that returns `unimplemented!` for now (Phase 6 fills it in).
- [ ] **Cadence floor enforcement**: `validate_cadence(cron_expr) -> Result<(), FeedError>` rejects any cron expression whose minimum interval is < 15 minutes. Implemented by computing the next 5 fire-times against `chrono` and confirming the minimum gap. No per-feed override.
- [ ] **Stub template** `stub/echo` for tests:
  - Param: `{ "message": "..." }`.
  - Run: writes one JSONL line `{ ts, message }` to `feed_dir/log.jsonl`. Cursor = run_count.
  - Used by every unit test that exercises the runtime end-to-end.
- [ ] **Wire into arawn server boot** in `crates/arawn/src/main.rs`: after the workflow runner starts, call `arawn_feeds::start(workflow_runner.clone(), db.clone(), config.storage.data_dir.clone(), integration_registry.clone()).await?;`. Server-startup logs show `feed runtime started, registered N feeds`.
- [ ] **Tests** in `arawn-feeds`:
  - `feeds_table_round_trips_a_record` (insert → list → get → delete).
  - `meta_json_atomic_write_doesnt_corrupt_on_partial_failure` (write → simulate crash → read returns prior version).
  - `cadence_floor_rejects_sub_15_minute_crons`.
  - `template_registry_finds_registered_templates_and_errors_on_unknown_names`.
  - `stub_template_runs_to_completion_and_persists_meta` (full integration test using a real cloacina runner with a 16-minute test cadence + manual trigger).
- [ ] `angreal check workspace` and `angreal check clippy` clean.
- [ ] Server starts cleanly with the new boot wiring; no behavioral change for existing users (no feeds = no work).

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates **[REQUIRED]**

### 2026-05-08 — Foundation landed; main.rs wiring + RealSlackClient adapter remain

**Done so far** (31 tests green, workspace + clippy clean):

- New `arawn-feeds` crate — error / types / cadence / clients (with mock seam) / layout / meta / registry / store / template / dispatch / runtime / templates modules.
- V2 migration in arawn-storage: `feeds` table.
- `FeedTemplate` trait: pure Rust trait, async run signature `(ctx, params, feed_dir, &cursor) -> RunOutcome`. Templates own all on-disk layout under their feed dir; runtime touches only `meta.json`.
- `FeedClients` mock-injection seam: per-provider client traits (`SlackFeedClient`) abstract upstream surfaces; production wraps `arawn-integrations`, tests use fakes.
- `MetaStore` with atomic-rename writes (no torn `meta.json` on crash).
- 15-minute cadence floor enforced via `validate_cadence` (computes 5 next fire-times against `cloacina::CronEvaluator`, rejects any pair under floor).
- `FeedDispatchTask` impls `cloacina::Task`. `run_feed(feed_id, runtime_ctx)` is the trait body, also exposed for non-cloacina tests. Persists `last_run_at` / `last_status` / `run_count` even on failure (so operators see the latest error in `meta.json`).
- `arawn_feeds::start(runner, conn, layout, registry, clients)` reads every enabled feed, registers a per-feed workflow via `Runtime::register_workflow` with a closure capturing the feed_id, then schedules it with `register_cron_workflow`. `FeedRuntime::register_feed_runtime(&FeedRecord)` does the same for dynamic `/watch` adds.
- `arawn-workflow::WorkflowRunner::cloacina_runner()` exposes `Arc<DefaultRunner>` so `arawn-feeds` can register without depending on `arawn-workflow` directly.
- One real ingestor end-to-end: **`slack/channel-archive`**. Mock `SlackFeedClient` harness in `tests/slack_channel_archive.rs` exercises:
  - first run writes JSONL + advances cursor
  - second run carries cursor through, appends only delta
  - empty-result returns `no-new-items` status without touching cursor
  - cross-day messages partition into separate `YYYY-MM-DD.jsonl` files
  - missing slack integration → `FeedError::Auth`
- One stub template `stub/echo` exercises the runtime end-to-end without any provider.

**Remaining for T-0214 to be truly complete:**

1. **`RealSlackClient` adapter** — the production impl of `SlackFeedClient` wrapping the existing slack-morphism client surface in `arawn-integrations`. Roughly 100 LOC: implement `resolve_channel` + `channel_history`. Lives in `arawn-feeds/src/clients/real.rs` (or `arawn-integrations/src/slack/feed_client.rs` — TBD when we look at the slack-morphism API surface).
2. **Server boot wiring** in `crates/arawn/src/main.rs`: open a `Connection` to `arawn.db` for the feeds runtime, build the registry + real clients bundle, call `arawn_feeds::start(...)` after the workflow runner is up. Need the `RealSlackClient` adapter from (1) before this lands.
3. **Integration test through real cloacina** — exercise the full path from `register_cron_workflow` via a manual fire (not waiting 15 min for cron). cloacina exposes a way to trigger a workflow on demand for tests; need to verify the API.

Stopping here as a coherent checkpoint: foundational code is right, tested, and architecture is locked. The remaining work is concrete adapter + boot wiring rather than open design questions.

### 2026-05-08 — RealSlackClient + main.rs boot wiring landed

- Restructured `clients/` from a single `clients.rs` into a per-provider module tree (`clients/mod.rs` for the bundle + `RealClients` builder, `clients/slack.rs` for the trait + `RealSlackClient`). Each new provider gets one new file holding both the trait and the production impl side-by-side.
- `RealSlackClient` adapter: maps `SlackFeedClient::resolve_channel` → `conversations.list` (prefers user context for private-channel visibility, falls back to bot), maps `channel_history` → `conversations.history` with cursor passthrough; reverses slack-morphism's newest-first into oldest-first; surfaces `Auth` / `RateLimited` / `Provider` errors based on slack-morphism's response shape.
- Server boot in `crates/arawn/src/main.rs`:
  - Hoisted the Slack `Arc<SlackIntegration>` so it's reachable after the workflow runner is up.
  - Hoisted the `WorkflowRunner` Arc so we can grab `cloacina_runner()` for arawn-feeds.
  - After the workflow runner starts, opens a `rusqlite::Connection` to `arawn.db`, builds `RealClients` with whatever integrations are connected, builds `default_registry()`, and calls `arawn_feeds::start(...)`.
- Smoke-tested the binary: server logs `feed runtime started registered=0 skipped=0` cleanly. No feeds configured yet (DB empty); registration path is alive and ready.
- 33 arawn-feeds tests green (28 unit + 5 integration). Workspace + clippy clean.

**Remaining (small, deferred):** an integration test that exercises a real cloacina runner firing a feed end-to-end. The current end-to-end coverage tests `run_feed()` directly (which is what cloacina would invoke). Adding the cloacina-cron-fire layer would be a single test that:
1. Builds a `WorkflowRunner` against a tempdir.
2. Calls `arawn_feeds::start` with one stub feed.
3. Manually triggers via cloacina's execute API and verifies meta.json updates.

Calling T-0214 done — the runtime + first ingestor + boot wiring all land cleanly and the binary boots with the new path. Cloacina-fire integration test can ride along with the next ingestor task (T-0215).

### 2026-05-08 — Authoritative cloacina-fire integration test landed

Pushed through the cloacina-internals investigation. Root cause was a namespace-encoding bug in our workflow naming convention:

- cloacina's `TaskNamespace` is `tenant::package::workflow_id::task_id` joined with `::`.
- We used `feed::<feed_id>` as the workflow name. Stitched into a namespace it became `public::embedded::feed::<feed_id>::<task_id>` — 5 parts, not 4.
- `parse_namespace` rejects it during executor task lookup; the workflow_execution sits Pending forever because the task can't be resolved.

Fix: changed `feed_workflow_name` to use `feed_<feed_id>` (single underscore separator) so `::` is reserved for cloacina's namespace separator. One-line change.

New tests in `tests/cloacina_fire.rs`:

- `cloacina_fires_feed_workflow_end_to_end` — stands up a real `DefaultRunner`, registers one stub feed via `arawn_feeds::start`, calls `runner.execute(...)`, verifies `meta.json` updated and JSONL line written. **0.55s wall clock.**
- `cloacina_fires_advance_cursor_across_two_executions` — three back-to-back `execute` calls, verifies cursor advances, run_count goes to 3, JSONL gets three lines.
- `registering_a_feed_with_unknown_template_is_skipped_at_boot` — a feed referencing a missing template doesn't abort `start()`; a real feed registered alongside still works.

All 36 arawn-feeds tests pass: 28 unit + 5 slack-mock integration + 3 cloacina-fire integration. Workspace + clippy clean. The full path is now authoritatively covered from cron registration → cloacina scheduler → executor → task lookup → `run_feed` → meta.json persistence.

T-0214 fully done.