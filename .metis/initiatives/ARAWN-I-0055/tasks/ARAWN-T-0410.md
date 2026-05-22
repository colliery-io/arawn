---
id: t-f-t-0394-regression-test
level: task
title: "T-F: T-0394 regression test — assert calendar tools survive iter-2+ filter"
short_code: "ARAWN-T-0410"
created_at: 2026-05-22T16:36:05.000000+00:00
updated_at: 2026-05-22T16:36:05.000000+00:00
parent: ARAWN-I-0055
blocked_by: [ARAWN-T-0406]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0055
---

# T-F: T-0394 regression test — assert calendar tools survive iter-2+ filter

## Parent Initiative

[[ARAWN-I-0055]]

## Objective

Write a unit test that reproduces the structural failure mode from ARAWN-T-0394 and asserts the post-T-B fix holds. Close T-0394 against this test as the artifact.

## Acceptance Criteria

- [ ] New unit test in `crates/arawn-engine/src/query_engine.rs::mod tests`:
  - Test name: `t_0394_calendar_tools_survive_filter_after_iter_1_when_calendar_capability_connected`
  - Setup:
    - Build a `ToolRegistry` with `calendar_upcoming` (category: `Calendar`), `weekly_run` (category: `Ceremony`), `web_fetch` (category: `Web`).
    - Build a `Session` with messages.len() > 2 — must trigger the post-iter-1 filter activation (not the early-return path).
    - User message: literal text from T-0394 — "Switch to `personal`. Bob replied to my catch-up email — he's open Tue/Wed mornings or Thu after 2 next week. Pick a 30-min slot..."
    - Capability set: `["calendar"]` connected.
    - `ModelLimits.context_window = 32_000` (force the filter path, no T-E bypass).
  - Assertion: filtered tool list contains `calendar_upcoming`. This is the exact case that failed pre-fix.
- [ ] Companion test: same setup but capability set is empty. Assert `calendar_upcoming` is NOT in the filtered list. Confirms the capability gate works both ways.
- [ ] Doc comment on the test points back to ARAWN-T-0394 with a one-line summary of the original failure.
- [ ] ARAWN-T-0394 transitioned to `completed` with a closing status update pointing at this regression test.
- [ ] `cargo test --workspace --lib` green.

## Implementation Notes

- This is the last task before initiative close — it depends on T-B (the actual fix) landing first.
- The test belongs in `query_engine.rs` (not a separate test crate) so it lives with the filter code it's protecting.
- Use the same identifiers and message strings as the original failure transcript so future readers can trace it back to T-0394.

## Status Updates

*To be added during implementation*
