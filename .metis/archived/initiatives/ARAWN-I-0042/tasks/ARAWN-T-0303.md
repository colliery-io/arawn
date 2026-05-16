---
id: weekly-agent-tools-weekly-family
level: task
title: "Weekly agent tools (weekly_* family)"
short_code: "ARAWN-T-0303"
created_at: 2026-05-16T16:38:00.660848+00:00
updated_at: 2026-05-16T16:57:26.930718+00:00
parent: ARAWN-I-0042
blocked_by: [ARAWN-T-0302]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0042
---

# Weekly agent tools

## Parent Initiative

[[ARAWN-I-0042]]

## Objective

Seven agent-callable tools mirroring retro/daily, adapted for
the Monday confirmation flow. Three priority-mutation tools
(`confirm/reject/add`) wrap the service methods from
[[ARAWN-T-0302]].

## Acceptance Criteria

## Acceptance Criteria

- [ ] New tools in `crates/arawn-engine/src/tools/weekly.rs`:
      - `weekly_run`, `weekly_current` (read-only),
        `weekly_list_items` (read-only), `weekly_list_priorities`
        (read-only), `weekly_confirm_priority`,
        `weekly_reject_priority`, `weekly_add_priority`.
- [ ] All carry `ToolCategory::Ceremony`. Query-engine keyword
      gate gains `weekly | priority | priorities | week`.
- [ ] Registered in `main.rs` under `weekly_actually_enabled`
      (handled by [[ARAWN-T-0304]]).
- [ ] Per-tool unit tests in the pattern of the daily tool tests.

## Implementation Notes

### Technical Approach

1. Copy `tools/daily.rs` verbatim and adapt names.
2. `weekly_list_priorities` returns `PriorityDto` array
   (`source` discriminator) so the agent can group confirmed vs
   candidate.
3. Mutating tools follow the retro/daily pattern.

### Dependencies

- Blocked by [[ARAWN-T-0302]].
- Co-lands with [[ARAWN-T-0304]].

### Risk Considerations

- **Tool count**: 5 retro + 5 daily + 7 weekly = 17 ceremony
  tools. If selection thrashes, consolidation (single tool with
  `action` parameter per family) gets reopened.

## Status Updates

### 2026-05-16 — seven weekly tools shipped

- New `crates/arawn-engine/src/tools/weekly.rs` (~600 lines):
  WeeklyRunTool, WeeklyCurrentTool, WeeklyListItemsTool,
  WeeklyListPrioritiesTool, WeeklyConfirmPriorityTool,
  WeeklyRejectPriorityTool, WeeklyAddPriorityTool.
- All carry `ToolCategory::Ceremony`. Keyword gate extended
  with `weekly | week | priority | priorities`.
- Re-exported from `arawn-engine::lib` next to daily.
- 13 unit tests pass. Full arawn-engine lib suite: 657 (was
  644).

Tool registration in `main.rs` happens under T-0304's
`weekly_actually_enabled` guard.

Completed 2026-05-16.