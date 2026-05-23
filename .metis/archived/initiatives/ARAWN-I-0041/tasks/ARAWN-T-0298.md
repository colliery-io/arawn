---
id: daily-agent-tools-daily-family
level: task
title: "Daily agent tools (daily_* family)"
short_code: "ARAWN-T-0298"
created_at: 2026-05-16T14:00:00+00:00
updated_at: 2026-05-16T16:17:36.027528+00:00
parent: ARAWN-I-0041
blocked_by: [ARAWN-T-0299]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0041
---

# Daily agent tools

## Parent Initiative

[[ARAWN-I-0041]]

## Objective

Five agent-callable tools mirroring the `retro_*` family from
[[ARAWN-T-0293]], adapted for daily: surface today's tablet, list
items, toggle todo done, and let the user add a fresh todo via the
user-write path.

## Acceptance Criteria

## Acceptance Criteria

- [ ] New tools in `crates/arawn-engine/src/tools/daily.rs`:
      - `daily_run { }` — fires `CeremonyService::run("daily")`. Same
        Generated/Skipped shape as `retro_run`.
      - `daily_current { }` (read-only) — current date's daily
        tablet, or `null` if not yet generated.
      - `daily_list_items { tablet_id, section_key? }` (read-only)
        — proxies `list_items`. Description tells the agent to
        quote `citation_id` values verbatim (judge grounding).
      - `daily_patch_item { item_id, patch }` — proxies
        `patch_item`. Used to toggle todos done.
      - `daily_add_todo { body }` — user-write path. Constructs an
        `AddItemRequest` against today's tablet, section_key
        `todos`, kind `todo`. Also writes a row to
        `ceremony_todos_rolling` so future daily generations see it.
- [ ] All five carry `ToolCategory::Ceremony` (already gated on
      `retro`/`ceremony`/`standup`/`diary` mentions in T-0293;
      append `daily` and `today` to that list in `query_engine.rs`).
- [ ] Registered in `main.rs` alongside the retro tools.
- [ ] Per-tool unit tests covering validation paths, error mapping,
      schema shape. Same shape as the eight tests under
      `tools::ceremony::tests::*`.

## Implementation Notes

### Technical Approach

1. Reuse the `RetroXxxTool` skeleton — each daily tool is a near-
   verbatim copy. Worth a shared `#[macro_rules]` or just
   straightforward duplication; lean toward duplication unless the
   five tools' bodies grow.
2. `daily_add_todo` is the only new shape vs. retro. It needs:
   - Compute today's daily tablet id via
     `CeremonyService::get_by_period("daily", today)`.
   - `add_item` with section_key `"todos"`, kind `ItemKind::Todo`.
   - Also `INSERT INTO ceremony_todos_rolling` so rollover_heat
     (retro detector) sees it next week. This is a small extension
     point on `CeremonyService` — likely a new `add_rolling_todo`
     helper.
3. Query-engine keyword gates: ceremony category currently fires on
   `retro|ceremony|standup|diary`. Add `daily|today|brief` so
   prompts like "what's on for today" pull these tools in.

### Dependencies

- Blocked by [[ARAWN-T-0299]] — needs `DailyCeremony` wired into
  the running binary so the service has a registered plugin to run.
- Builds on the agent-tool plumbing shipped in [[ARAWN-T-0293]].

### Risk Considerations

- **Tool count bloat**: 5 retro + 5 daily = 10 ceremony tools the
  agent can pick from. If selection thrashes, the consolidation
  hint from T-0293's risk section (collapse into single
  `retro`/`daily` tools with `action` parameter) lands here too.

## Status Updates

### 2026-05-16 — five daily tools shipped

- New `crates/arawn-engine/src/tools/daily.rs` (~470 lines) with
  `DailyRunTool`, `DailyCurrentTool`, `DailyListItemsTool`,
  `DailyPatchItemTool`, `DailyAddTodoTool`.
- `DailyAddTodoTool` looks up today's tablet, calls `add_item`
  with section_key=`todos`, and *also* inserts a row into
  `ceremony_todos_rolling` (new
  `CeremonyService::add_rolling_todo` helper) so retro's
  `rollover_heat` detector sees the new todo next week.
- `ToolCategory::Ceremony` keyword gate extended with
  `daily | today | brief`.
- Five tools registered in `main.rs` under `daily_actually_enabled`.
- 9 unit tests pass (5 validation + schemas + `daily_current`
  positive case + end-to-end `daily_add_todo` verifying both
  inserts). Full `arawn-engine --lib` suite (644 tests) green.

Completed 2026-05-16.