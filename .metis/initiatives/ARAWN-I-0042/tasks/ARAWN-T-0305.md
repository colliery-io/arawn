---
id: weekly-uat-scenario-fixture-seeder
level: task
title: "Weekly UAT scenario + fixture + seeder — Monday confirm flow LLM-judged"
short_code: "ARAWN-T-0305"
created_at: 2026-05-16T16:38:11.674109+00:00
updated_at: 2026-05-16T16:38:11.674109+00:00
parent: ARAWN-I-0042
blocked_by:
  - ARAWN-T-0303
  - ARAWN-T-0304
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0042
---

# Weekly UAT scenario + fixture + seeder

## Parent Initiative

[[ARAWN-I-0042]]

## Objective

Real-LLM-judged scenario for the weekly ceremony. The unique
content vs retro/daily is the Monday confirmation flow:
agent fires the weekly, sees candidate priorities, then
confirms a subset.

## Acceptance Criteria

- [ ] `crates/arawn-tests/tests/fixtures/uat/weekly-ceremony.json`
      with three workstreams + a handful of recent gmail/slack
      rows so the LLM has signals to surface as deadlines /
      attention items.
- [ ] `crates/arawn-tests/tests/uat_weekly_seed.rs` writing:
      - Calendar events across the current ISO week (Mon–Sun)
        via `CalendarEventProjection`.
      - A prior weekly tablet with 2 un-done items (for the
        `inbound` section).
      - A prior retro tablet + diary (for `from_last_retro`).
      - 3+ rolling todos > 7d old un-done (for `rolling_todo_hot`).
      - Idempotent (`INSERT OR IGNORE` + projection upsert).
- [ ] `seed_weekly_ceremony: bool` field on `Scenario`, harness
      block, all six prior scenarios opt out.
- [ ] `fn weekly_ceremony_scenario() -> Scenario` with four
      turns:
      1. *user* "Run this week's prep and report the tablet id."
         *judge*: agent calls `weekly_run`, reports tablet_id.
      2. *user* "Use weekly_list_items grouped by section, quote
         each citation_id verbatim."
         *judge*: ≥1 item per section, citations verbatim.
      3. *user* "List the priority candidates with
         weekly_list_priorities. Pick the two most useful and
         confirm them via weekly_confirm_priority — **First**
         call each confirmation, **then** call
         weekly_list_priorities again to verify they show
         `source: confirmed`. Sequential, not parallel."
         *judge*: two confirmations, then a list call. The
         confirmed priorities have `source: confirmed`.
      4. *user* "Reject the remaining candidates via
         weekly_reject_priority, sequential calls."
         *judge*: rejects fired, list_priorities now shows only
         the confirmed pair.
- [ ] Registered in `all_scenarios()`.
- [ ] `angreal test uat --scenario weekly-ceremony` passes
      locally; `angreal test uat-judge` grades pass.

## Implementation Notes

### Technical Approach

1. Mirror `uat_daily_seed.rs` structure. The new sections are
   prior weekly + prior retro + rolling-todo-hot.
2. The 4-turn shape is necessary — turn 3 has the unique
   confirm-then-list interaction that the judge needs to grade.
3. Sequential-call enforcement on turns 3 + 4 uses the same
   `**First** / **Only after**` phrasing as retro Turn 3 /
   daily Turn 3.

### Dependencies

- Blocked by [[ARAWN-T-0303]] (tools) + [[ARAWN-T-0304]]
  (binary wiring) — without those, nothing for the scenario to
  call.

### Risk Considerations

- **Judge race**: known gemma4 quirk — even with explicit
  sequential phrasing, the model sometimes parallelises. Will
  show up in turns 3 + 4; judge gives partial credit. Document
  as known-and-acceptable in the status update.
- **Cost**: 4 turns ≈ 100s of real-LLM time. Still within
  reasonable bounds for UAT.

## Status Updates

*To be added during implementation*
