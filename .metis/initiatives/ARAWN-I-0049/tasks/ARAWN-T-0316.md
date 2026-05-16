---
id: uat-priority-completion-feedback
level: task
title: "UAT — priority-completion feedback loop"
short_code: "ARAWN-T-0316"
created_at: 2026-05-16T22:52:38.122290+00:00
updated_at: 2026-05-16T22:52:38.122290+00:00
parent: ARAWN-I-0049
blocked_by: [ARAWN-T-0312, ARAWN-T-0313, ARAWN-T-0314]
effort: M
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0049
---

# UAT — priority-completion feedback loop

## Parent Initiative

[[ARAWN-I-0049]]

## Objective

End-to-end LLM-judged proof that the todos refactor closes the
retro feedback loop. Seed a weekly with confirmed priorities →
mid-week daily marks some done via `todo_done` → run retro →
judge sees `priority_completion_ratio` reflect real state.

## Acceptance Criteria

- [ ] New scenario `priority_completion_feedback_scenario` in
      `crates/arawn-tests/tests/uat.rs`.
- [ ] Seed: a weekly tablet with N confirmed priorities (todos),
      a daily mid-week, M < N of the priority todos marked done
      via `todo_done`.
- [ ] Agent prompt drives: run a retro → expect
      `priority_completion_ratio` detector to fire with ratio
      M/N (not 0/N, not skipped).
- [ ] Judge assertions: (a) ratio matches seeded M/N, (b) retro
      narrative references the completed priorities by body,
      (c) un-done priorities show up in `rollover_heat` if past
      their due window.
- [ ] Existing retro / daily / weekly UATs continue to pass
      against refactored gather paths (regression gate).

## Implementation Notes

### Technical Approach

- Seed module: new `uat_priority_completion_seed.rs` alongside
  the existing ceremony seeds.
- Reuse the weekly + retro fixtures as a starting point; layer
  the `todo_done` mutations as explicit operations in the
  scenario script.

### Dependencies

- Blocked by [[ARAWN-T-0312]] (cutover), [[ARAWN-T-0313]] (tools),
  [[ARAWN-T-0314]] (TUI not strictly needed for UAT but bundled
  so a single Phase-2 verification covers everything).

### Risk Considerations

- Judge flake: prior LLM-judged UATs occasionally need prompt
  tightening on edge cases. Build the scenario with explicit
  expected-ratio numbers so the judge has unambiguous signal.

## Status Updates

*To be added during implementation*
