---
---
id: uat-scenario-4-weeks-of-synthetic
level: task
title: "UAT scenario — 4 weeks of synthetic tablets → retro plugin asserts"
short_code: "ARAWN-T-0291"
created_at: 2026-05-15T23:46:00.496411+00:00
updated_at: 2026-05-15T23:46:00.496411+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0043
---

# UAT scenario — synthetic 4-week retro run

## Goal
End-to-end test that seeds 4 weeks of synthetic tablets + rollups + activity, runs the retro plugin with a fake clock, asserts: pattern rows fire correctly, composed items all carry citations, diary upsert persists, RPC surface returns expected shapes.

## Reference
I-0043 Implementation Plan stage 1 step 11.

## Acceptance
- New UAT test under `crates/arawn-tests/tests/ceremonies_retro.rs` (matching the existing UAT pattern).
- Test fixtures: 4 ISO weeks of `ceremony_tablets` (daily + weekly), `ceremony_priorities` with mixed done/undone, `ceremony_activity_rollup` rows, a couple of rolled-over todos.
- Steps:
  1. seed DB
  2. set fake clock to a Friday 16:00
  3. invoke `CeremonyRunner::run_once("retro")`
  4. assert: tablet generated with expected sections + ≥2 pattern rows + every composed item has a citation
  5. drive `ceremonies.upsert_diary` over the RPC
  6. assert: `ceremony_diary` row exists, tablet status = `reviewed`, `EngineEvent::Ceremony(DiaryUpdated)` was published
- Bootstrap variant: re-seed with only 1 week of history; assert pattern section is absent, retro still ships, no LLM errors.

## Out of scope
Real-LLM smoke test — UAT uses MockLlmClient. Real-LLM exercise lives in the long-running UAT job once retro ships behind the flag.
