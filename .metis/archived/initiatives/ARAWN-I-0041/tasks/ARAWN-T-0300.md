---
id: daily-uat-scenario-fixture-seeder
level: task
title: "Daily UAT scenario + fixture + seeder — LLM-judged end-to-end"
short_code: "ARAWN-T-0300"
created_at: 2026-05-16T14:00:00+00:00
updated_at: 2026-05-16T16:31:47.069615+00:00
parent: ARAWN-I-0041
blocked_by: [ARAWN-T-0298, ARAWN-T-0299]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0041
---

# Daily UAT scenario + fixture + seeder

## Parent Initiative

[[ARAWN-I-0041]]

## Objective

Add a real-LLM scenario to `angreal test uat` that exercises the
daily ceremony end-to-end through the binary's WS-RPC + daily_*
agent tools. Mirrors the retro UAT structure shipped in
[[ARAWN-T-0294]].

## Acceptance Criteria

## Acceptance Criteria

- [ ] New fixture `crates/arawn-tests/tests/fixtures/uat/daily-ceremony.json`
      with 2–3 workstreams + seeded gmail / slack rows that the
      attention adapter will surface as "new signals."
- [ ] New seeder `crates/arawn-tests/tests/uat_daily_seed.rs`
      writing:
      - A handful of `ceremony_todos_rolling` rows (un-done,
        carried over from yesterday).
      - One weekly tablet for the current ISO week with 2 confirmed
        priorities (so the `alignment` section has content).
      - Synthetic calendar events for "today" via the calendar
        projection table.
      - All inserts idempotent (`INSERT OR IGNORE`).
- [ ] New `seed_daily_ceremony: bool` field on `Scenario`, threaded
      through the harness next to `seed_retro_ceremony`. Existing
      scenarios opt out.
- [ ] `fn daily_ceremony_scenario() -> Scenario` with three turns:
      1. *user* "Run today's brief and report the tablet id."
         *judge*: agent calls `daily_run`, reports tablet id.
      2. *user* "List the items grouped by section. Quote each
         item's citation_id verbatim."
         *judge*: ≥4 items across at least 3 sections; citation_ids
         quoted verbatim in backticks.
      3. *user* "Add a todo: 'Email the SRE team about the proj-c RFC.'
         **First** call daily_add_todo, wait for the response.
         **Only after** it returns, call daily_list_items with
         section_key=`todos` and report the new item."
         *judge*: sequential tool calls; new todo appears with the
         literal body string.
- [ ] Registered in `all_scenarios()`.
- [ ] `angreal test uat --scenario daily-ceremony` passes locally;
      `angreal test uat-judge` grades pass.

## Implementation Notes

### Technical Approach

1. Copy the structure of `uat_retro_seed.rs` verbatim and adapt the
   inserts. Most of the schema work (ISO week computation,
   Monday/Sunday bracketing) is shared and could move into a
   `uat_ceremony_seed_common.rs` helper if duplication gets noisy.
2. Calendar fixture: the seeder needs to insert rows into the
   calendar projection table. Confirm the exact table + projection
   crate before writing — `arawn-feeds` calendar projection lives
   in `crates/arawn-feeds/migrations/`.
3. Turn 3 parallel-call protection: use the same
   `**First** ... **Only after**` phrasing from
   `tag-promoter-cycle` and the retro scenario's Turn 3.

### Dependencies

- Blocked by [[ARAWN-T-0298]] (agent tools) and [[ARAWN-T-0299]]
  (binary wiring) — without those, nothing for the scenario to
  call.

### Risk Considerations

- **Judge race on Turn 3**: known issue with gemma4 — even with
  explicit sequential phrasing, the model sometimes parallelises.
  Document as a known characteristic; judge gives partial credit.
- **Calendar projection coupling**: if the calendar table schema
  changes in arawn-feeds, this seeder breaks. Worth a compile-time
  check (use the projection's typed writer rather than raw SQL
  where possible).

## Status Updates

### 2026-05-16 — UAT shipped and graded PASS

- New fixture + seeder + scenario shipped per spec. Calendar
  events written via `CalendarEventProjection` typed writer
  (avoids raw-SQL coupling to the schema).
- 2 seeder unit tests pass. Full uat-target build clean.
- **Real-LLM run** (gemma4:31b-cloud): PASS, completion=4/5,
  quality=4/5. Turn 1+2 scored 5/5/5/5. Turn 3 docked on
  adherence (model parallelised daily_add_todo +
  daily_list_items — same gemma4 quirk documented on retro
  Turn 3). The new todo still surfaced correctly so the
  ceremony engine + agent tools work end-to-end.

Completed 2026-05-16.