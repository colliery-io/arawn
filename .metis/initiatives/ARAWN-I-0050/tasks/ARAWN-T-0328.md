---
id: end-to-end-smoke-test-bind-tick
level: task
title: "End-to-end smoke test — bind → tick → assert"
short_code: "ARAWN-T-0328"
created_at: 2026-05-18T14:34:06.363231+00:00
updated_at: 2026-05-18T18:27:17.159090+00:00
parent: ARAWN-I-0050
blocked_by: [ARAWN-T-0327]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0050
---

# End-to-end smoke test — bind → tick → assert

## Parent Initiative

[[ARAWN-I-0050]]

## Objective

Unit-test-only smoke that proves the full chain works end-to-end
against a fake `GithubFeedClient`: bind a workstream → run a tick
→ projection tables + workstream KB reflect the seeded data.

## Acceptance Criteria

- [x] New integration test
      `crates/arawn-feeds/tests/github_repo_mirror_smoke.rs`.
- [x] `full_repo_mirror_round_trips_through_projection_store`:
      seeds a `FakeGithub` with 2 commits + 2 issues + 1 PR + 2
      comments; runs one `RepoMirrorTemplate` tick; asserts
      JSON files at `<owner>/<repo>/<kind>/<id>.json`; runs
      `project_feed_dir` and asserts row counts +
      FTS hits (panic, ship) + metadata round-trip on one
      commit + kind discriminator on one comment + cursor
      advance across all four kinds.
- [x] `second_tick_with_no_new_data_yields_no_new_items`:
      proves cursor pickup — after a tick with data and then
      a tick with no new commits, status is `no-new-items`
      and the cursor stays put.
- [x] `dispatch_with_empty_feed_dir_is_a_noop`: defensive
      regression for the new dispatch arm — empty feed_dir
      doesn't error, all four tables stay empty.
- [-] Scenarios for "bind path drops repo on org bind" and
      "repo bind rejected when org exists" are covered in
      [[ARAWN-T-0326]]'s tool-level tests against a real
      `Store`; not duplicated here.
- [-] Scenarios for "org bind fans out to N repo feeds via
      list_org_repos" need access to `expand_github_org`
      which lives in `arawn` (the binary). The smoke covers
      the data path through the template + dispatch; the
      bind→hook→expand path is checked at workstream-tool
      unit level (T-0327's sweep test) and would be a
      bin-level integration test if needed. Filed as
      follow-up only if a real-API UAT for I-0050 ever lands.
- [x] No real network — entire smoke runs against the
      `FakeGithub`. 3 tests, all green.

## Status Updates

### 2026-05-18 — smoke shipped

- Three integration tests in `arawn-feeds/tests/`. The full
  round-trip test exercises template → atomic file writes →
  projection dispatch → ProjectionStore writes → FTS search
  + metadata round-trip end-to-end.
- The cross-tick cursor test proves the template's stateful
  cursor logic survives a round-trip through the JSON
  round-trip of `RunOutcome.cursor`.
- Workspace 1895/0 (1892 → 1895, +3).

## Implementation Notes

### Technical Approach

- Pattern-match the I-0045 template tests (`FakeGithub` impl,
  `WithFakeGithub` `FeedClients`) but seed projection +
  workstream state too so the bind hooks fire end-to-end.

### Dependencies

- Blocked by [[ARAWN-T-0327]] — every other task in the
  initiative needs to be in place.

### Risk Considerations

- Cross-crate seeding can be fiddly. If the smoke needs more
  than the existing arawn-feeds test harness supports, file in
  arawn-tests like the I-0049 UAT scenarios.