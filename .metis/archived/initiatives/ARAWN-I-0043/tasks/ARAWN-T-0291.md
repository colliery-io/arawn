---
id: uat-scenario-4-weeks-of-synthetic
level: task
title: "UAT scenario — 4 weeks of synthetic tablets → retro plugin asserts"
short_code: "ARAWN-T-0291"
created_at: 2026-05-15T23:46:00.496411+00:00
updated_at: 2026-05-16T03:16:33.693002+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


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
## Status Updates

**2026-05-16 — implementation landed.**

End-to-end UAT lives at `crates/arawn-ceremonies/tests/retro_uat.rs` (integration test, not in `arawn-tests` — keeps the heavy deps out of the ceremonies UAT's compile path).

Two scenarios, both green:

**`uat_4_week_retro_with_pattern_detection`** — the full happy path:
1. Seeds 3 prior weeks of rollup (so `workstream_neglect`'s 3-week lookback is satisfied) + this week's rollup with proj-b dropped (workstream_neglect fires for proj-b).
2. Seeds a daily tablet this Monday with a cite-able item, a weekly tablet with 1-of-3 priorities done (priority_completion fires below 0.5), and 3 un-done rolling todos created before this week (rollover_heat fires at threshold).
3. Subscribes to the event channel, dispatches the retro via `EngineDispatcher`, and drains events.
4. Asserts: `Generated` outcome with the expected tablet id, `TabletGenerated` + ≥1 `PatternDetected` events fired, exactly one composed item written, every composed item carries a non-NULL `citation_id`, ≥1 pattern row in `ceremony_patterns_detected`.
5. Calls `service.upsert_diary`, asserts diary body persisted verbatim, tablet status flipped to `reviewed`, and `DiaryUpdated` event fired.

**`uat_bootstrap_no_history_still_ships_retro`** — only this week's data:
1. No prior rollup history → `workstream_neglect` skipped by registry, `priority_completion` + `rollover_heat` fire-empty (no signal).
2. Dispatches retro; asserts `Generated` outcome, **zero** pattern rows, and the LLM's composed item still landed.
3. No errors, no panics — the bootstrap path renders cleanly.

**Fake-clock approach:** I built the fixtures relative to `Utc::now()`'s current ISO week rather than mocking the clock. The dispatcher reads `Utc::now()` and the plugin computes `period_key(now)`; the test seeds prior weeks and this-week data relative to the same `now`. Stable across whenever the test runs.

**Mock LLM:** `MockLlmClient` returns a hand-crafted JSON array citing the seeded daily item id, exercising the full parse path in `RetroCeremony::compose` without standing up a real LLM.

**Tests landing in this initiative across all tasks:**
- 80 lib unit tests (across error / events / engine / nightly / patterns / plugin / plugins / plugins::retro_detectors / registry / render / rollup / runner / service / types).
- 2 integration UATs (this task).
- Workspace check: clean.

Closes T-0291. With this, **I-0043 (engine + retro plugin) tier-1 stage is fully shipped.**