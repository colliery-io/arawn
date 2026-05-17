---
id: uat-priority-completion-feedback
level: task
title: "UAT — priority-completion feedback loop"
short_code: "ARAWN-T-0316"
created_at: 2026-05-16T22:52:38.122290+00:00
updated_at: 2026-05-17T18:57:29.604011+00:00
parent: ARAWN-I-0049
blocked_by: [ARAWN-T-0312, ARAWN-T-0313, ARAWN-T-0314]
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

- [x] New scenario `priority_completion_feedback_scenario` in
      `crates/arawn-tests/tests/uat.rs`. Three turns + judge
      expectations spelled out.
- [x] Seed reuses `seed_retro_ceremony=true` — the existing seeder
      already populates 3 confirmed open `kind='weekly_priority'`
      todos on the current weekly tablet (no done_at). The agent
      drives M=1 via `todo_done` in turn 2, hitting ratio 1/3 =
      0.33 (below the detector's 0.5 threshold so it fires).
- [x] Agent prompt drives: turn 1 lists priorities via `todo_list`,
      turn 2 marks one done via `todo_done`, turn 3 runs a retro
      and inspects patterns via `retro_list_items section_key=
      patterns`. Judge expects `priority_completion_ratio` in the
      surfaced patterns.
- [x] Judge expectations include the explicit ratio (1/3) so a
      flake on ratio 0/3 (todo_done not propagated) is caught as
      a regression on the cutover plumbing.
- [x] All existing UATs (`retro`, `daily`, `weekly`,
      `signal-extraction-e2e`, etc.) continue to compile against
      the refactored gather paths. Unit suite 1814/0.
- [ ] Actually run the UAT against a real LLM and judge. Deferred
      to user — `angreal test uat -- UAT_SCENARIO=priority-completion-feedback`
      followed by `angreal test uat-judge`. Scaffolding is in place.

## Status Updates

### 2026-05-17 — scenario filed

- New scenario function `priority_completion_feedback_scenario`
  reuses the retro seeder so no new seed module needed. The
  retro seed already plants 3 confirmed open priorities; the
  agent's `todo_done` on one of them shifts ratio to 1/3 =
  0.33, which is below the detector's 0.5 threshold so the
  pattern fires.
- Three turns: list-priorities, mark-one-done, run-retro-and-
  inspect-patterns. Each turn's judge_expectation is explicit
  about what the LLM should call and what the response should
  contain.
- Wired into `all_scenarios()` so the UAT runner picks it up
  alongside the existing 7 scenarios.
- The actual judged run is a user-driven step (requires API key
  + cost) — scaffolding in place, deferred to user.

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