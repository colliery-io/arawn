---
id: phase-1-feed-runtime-cloacina
level: task
title: "Phase 1 — Feed runtime + cloacina wiring"
short_code: "ARAWN-T-0214"
created_at: 2026-05-07T00:42:18.450827+00:00
updated_at: 2026-05-07T03:42:04.573413+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


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

*To be added during implementation*