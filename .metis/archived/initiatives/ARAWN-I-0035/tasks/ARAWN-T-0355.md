---
id: brief-pipeline-integration-test
level: task
title: "Brief pipeline integration test — ceremony run + render end-to-end"
short_code: "ARAWN-T-0355"
created_at: 2026-05-19T13:00:00+00:00
updated_at: 2026-05-19T12:41:00.392579+00:00
parent: ARAWN-I-0035
blocked_by: [ARAWN-T-0349, ARAWN-T-0354]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0035
---

# Brief pipeline integration test — ceremony run + render end-to-end

## Parent Initiative

[[ARAWN-I-0035]]

## Objective

T-0349 and T-0354 ship the brief feature with unit + snapshot
coverage on the deterministic surface. What's missing: end-to-end
proof that running ceremonies (`daily_run` / `weekly_run`) writes
tablets the brief composer then renders correctly. This task
adds one integration test that exercises the full chain at the
library layer (no real LLM, no WS-RPC server).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] New integration test at
      `crates/arawn-ceremonies/tests/brief_pipeline.rs`.
- [ ] Test exercises this chain:
      1. Open a fresh test DB, build `PluginRegistry`.
      2. Register a scripted `daily` plugin pre-loaded with a
         few items across the canonical sections (`calendar`,
         `todos`, `attention`, `alignment`).
      3. Register a scripted `weekly` plugin pre-loaded with at
         least one priority and one item in another section
         (e.g. `calendar_shape`).
      4. Build `CeremonyService` + `EngineDispatcher`.
      5. `service.run("daily").await` → assert `Generated`.
      6. `service.run("weekly").await` → assert `Generated`.
      7. Read both tablets via `get_by_period` + `list_items`
         (+ `list_priorities` for weekly).
      8. Build `DailyView` + `WeeklyView` + `BriefView`.
      9. Call `render_brief(&view, now)`.
      10. Assert the rendered string contains seeded content
          from BOTH sections (e.g., the calendar item from daily
          AND the priority from weekly).
      11. Assert heading hierarchy: `## Today`, `## This week`,
          and `### Today's calendar` / `### Priorities` (proves
          the demotion is wired through real data).
- [ ] Second test case: pipeline with daily generated but
      weekly NOT generated → `BriefView { daily: Some, weekly:
      None }` → assert the rendered output contains the daily
      content and the "_(no weekly tablet" placeholder.
- [ ] No real LLM in scope: use a `ScriptedPlugin` pattern
      (mirroring the private one in `service.rs::tests`) that
      pre-loads `NewItem` values for `compose()` to return.
      Local declaration is fine — no need to extract a public
      test-support module unless we end up writing 2+ such tests
      elsewhere.
- [ ] `angreal test unit` green. `angreal check workspace` green.

## Implementation Notes

### Technical Approach

1. **Recreate the `ScriptedPlugin` test helper** at the top of
   the integration test file. ~50 LOC:
   ```rust
   struct ScriptedPlugin {
       kind: &'static str,
       items: Mutex<Vec<NewItem>>,
       period: String,
   }
   #[async_trait]
   impl Ceremony for ScriptedPlugin {
       fn kind(&self) -> &'static str { self.kind }
       fn period_key(&self, _now: DateTime<Utc>) -> String { self.period.clone() }
       fn default_schedule(&self) -> CronSchedule { CronSchedule::local("0 0 * * *") }
       async fn gather(&self, _ctx) -> Result<GatheredFacts, _> { Ok(GatheredFacts::new(json!({}))) }
       async fn compose(&self, _ctx, _facts) -> Result<Vec<NewItem>, _> {
           Ok(std::mem::take(&mut *self.items.lock().unwrap()))
       }
   }
   ```
2. **Seeded items** — pick one realistic-looking item per
   relevant section. Daily: `calendar` with "09:00 standup",
   `todos` with "ship I-0035 brief", `attention` with "RFC-0042
   waiting on you". Weekly: priority "Ship Phase 2 brief
   pipeline", item in `calendar_shape` with "7 meetings".
3. **Period keys** — pin to a stable date (e.g. "2026-05-19"
   for daily, "2026-W21" for weekly). The scripted plugin's
   `period_key` returns the pinned value regardless of `now`,
   so the test is wall-clock-independent.
4. **DBs** — reuse the existing `open_test_db()` pattern from
   `retro_uat.rs` (creates a tempdir + opens `Database::open`
   so refinery migrations run, then opens a `Connection` for
   `ConnHandle`).

### Dependencies

- `arawn-ceremonies` lib (already exports `CeremonyService`,
  `EngineDispatcher`, `PluginRegistry`, `BriefView`,
  `render_brief`, `DailyView`, `WeeklyView`).
- `arawn-storage` for `Database::open` (migrations).
- Trait `Ceremony` lives in the `plugin` module.

### Risk Considerations

- The scripted plugin's `kind()` must return `"daily"` (or
  `"weekly"`) to satisfy whatever section validation the
  dispatcher does. Verify against `EngineDispatcher`'s behavior
  if a "weekly" plugin is registered under the same registry —
  worst case, split into two registries.
- `EngineDispatcher` may require a `now` source that picks
  today's date. The scripted plugin overrides `period_key` so
  this should be moot, but watch for clock-dependent code in
  the dispatch path.

## Status Updates

### 2026-05-19 — Pipeline integration test shipped

- New `crates/arawn-ceremonies/tests/brief_pipeline.rs` with
  two test cases (~250 LOC, no real LLM).
- **Test 1 — `brief_pipeline_renders_daily_and_weekly_content`**:
  - Builds a fresh DB, registers two `ScriptedPlugin`s (one for
    "daily" pinned to `2026-05-19`, one for "weekly" pinned to
    `2026-W21`) in a single `PluginRegistry`. Both plugins
    pre-load `NewItem` values shaped like real `compose()`
    output, with the predicted `tablet_id`
    (`<kind>-<period_key>`) baked in.
  - Runs both ceremonies via `service.run("daily")` and
    `service.run("weekly")`; asserts both return
    `DispatchOutcome::Generated`.
  - Adds a priority to the weekly tablet via `add_priority` so
    the Priorities section has real content (priorities are a
    user-write surface, not a `compose()` output).
  - Reads tablets + items + priorities back through the service;
    builds `DailyView` + `WeeklyView` + `BriefView`.
  - Calls `render_brief` and asserts:
    - `# Brief — 2026-05-19` date header (daily wins over weekly).
    - `## Today` and `## This week` top-level headings present.
    - `### Today's calendar` and `### Priorities` — proves
      `demote_h2_to_h3` runs over real data, not just the
      hand-built fixtures in the unit tests.
    - Seeded content from BOTH sides surfaces: daily
      calendar / todo / attention items + weekly priority +
      calendar shape item.
- **Test 2 — `brief_pipeline_missing_weekly_renders_placeholder`**:
  - Same setup but only runs the daily ceremony.
  - Builds `BriefView { daily: Some(_), weekly: None }`.
  - Asserts daily content surfaces, weekly placeholder
    (`_(no weekly tablet`) fires, no weekly content leaks
    through.
- **One footgun caught during implementation:** items returned
  from `compose()` need the `tablet_id` field set to the
  predicted `<kind>-<period_key>` ID. Initial draft left it
  empty assuming the dispatcher would overwrite — got
  `FOREIGN KEY constraint failed`. Fix documented inline in the
  `composed()` helper.
- Helpers reused: `open_test_db()` (mirrors `retro_uat.rs`), the
  `ScriptedPlugin` impl (mirrors the private one in
  `service.rs::tests`). Not extracted to a public test-support
  module — only two callers across the crate, premature.
- Test results: 108 lib + 2 new integration + 2 prior retro_uat
  = 112 passing in `arawn-ceremonies`.
- `angreal check workspace` green.