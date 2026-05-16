---
id: retro-uat-scenario-fixture-llm
level: task
title: "Retro UAT scenario + fixture — LLM-judged end-to-end"
short_code: "ARAWN-T-0294"
created_at: 2026-05-16T03:22:46.609267+00:00
updated_at: 2026-05-16T03:22:46.609267+00:00
parent: ARAWN-I-0043
blocked_by:
  - ARAWN-T-0292
  - ARAWN-T-0293
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0043
---

# Retro UAT scenario + fixture — LLM-judged end-to-end

## Parent Initiative

[[ARAWN-I-0043]]

## Objective

Add a real LLM-judged scenario to `angreal test uat` that exercises the retro
ceremony end-to-end through the binary's RPC + agent-tool surface. T-0291
shipped a mock-LLM in-crate UAT; this is its end-to-end counterpart, judged by
`angreal test uat-judge`.

## Acceptance Criteria

- [ ] `crates/arawn-tests/tests/fixtures/uat/retro-ceremony.json` exists with synthetic seed:
  - [ ] 2–3 workstreams (e.g., `proj-a`, `proj-b`, `proj-c`).
  - [ ] 5–7 days of feed rows distributed across workstreams (the existing fixture loader builds daily tablets from this).
  - [ ] At least 3 prior weekly rollup entries (so `WorkstreamNeglectDetector` has enough history to fire).
  - [ ] A prior weekly priority row with done/confirmed ratio < 0.5 (triggers `PriorityCompletionDetector`).
  - [ ] A handful of un-done `ceremony_todos_rolling` rows with `last_seen` in the current week (triggers `RolloverHeatDetector`).
- [ ] `fn retro_ceremony_scenario() -> Scenario` in `crates/arawn-tests/tests/uat.rs` with turns:
  1. *user* "Run this week's retro and tell me the tablet id." *judge*: agent calls `retro_run`, reports a tablet id.
  2. *user* "Read out the items the engine flagged — group by section, include the citation id for each." *judge*: agent calls `retro_list_items`, reports ≥3 items across ≥2 sections; each reported item has a non-empty citation_id grounded in the seed (citations should match `sig-*`/`pat-*`/`event-*` ids from the fixture).
  3. *user* "Save this diary entry: 'Felt focused; proj-c starved this week.'" *judge*: agent calls `retro_save_diary`; subsequent `retro_current` shows status flipping to `reviewed`.
- [ ] Mechanical thresholds: `min_files_created: 0`, `min_workflows_created: 0`, `min_memory_entities: 0`, `max_tool_errors: 1`.
- [ ] Registered in `all_scenarios()`.
- [ ] `angreal test uat` runs the scenario locally against a real LLM and `angreal test uat-judge` grades it pass.

## Implementation Notes

### Technical Approach

1. Fixture builder pattern: copy `signal-extraction-e2e.json` shape but extend the loader (if needed) to seed `ceremony_activity_rollup`, `ceremony_priorities`, `ceremony_todos_rolling`. The seed code lives in `crates/arawn-tests/tests/fixtures/` or wherever `seed_fixture` is consumed — confirm before extending.
2. Citation grounding is the key judge criterion. The seed must use stable, easy-to-spot ids (`sig-001`, `event-007`) so the judge can see them surfaced in the agent's output.
3. The two existing detectors' bootstrap-skipping behavior matters: ensure history depth ≥ 3 weeks so `WorkstreamNeglectDetector` is not skipped.

### Dependencies

- Blocked by [[ARAWN-T-0292]] (RPC) and [[ARAWN-T-0293]] (agent tools) — without them the agent has nothing to call.
- Closes the loop on the user's original ask: "set up the test scenario so we have an actual LLM judged UAT test."

### Risk Considerations

- **Judge brittleness**: LLM judges sometimes give credit for verbose narration even when citation ids weren't surfaced. The judge prompt should explicitly require the agent to *quote* citation ids verbatim.
- **Seed loader extension**: If the existing fixture loader doesn't know how to populate the ceremony tables, the loader needs extension first — flag as a sub-step rather than a separate task unless it gets large.
- **Cost**: Each UAT run is a real LLM call. Keep turns to 3, no exploration prompts.

## Status Updates

*To be added during implementation*
