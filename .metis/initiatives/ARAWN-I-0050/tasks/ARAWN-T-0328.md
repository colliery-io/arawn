---
id: end-to-end-smoke-test-bind-tick
level: task
title: "End-to-end smoke test — bind → tick → assert"
short_code: "ARAWN-T-0328"
created_at: 2026-05-18T14:34:06.363231+00:00
updated_at: 2026-05-18T14:34:06.363231+00:00
parent: ARAWN-I-0050
blocked_by: [ARAWN-T-0327]
effort: S
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

- [ ] New integration test under
      `crates/arawn-feeds/tests/github_repo_mirror_smoke.rs`
      (or `arawn-tests` if cross-crate seeding is needed).
- [ ] Test scenario:
      1. Bind `github:repo:openai/codex` to workstream `pat`.
      2. Fake client returns canned commits, issues, PRs,
         comments for that repo.
      3. Run one template tick.
      4. Assert: projection store has rows of all four kinds;
         `find_workstream_for_feed("github-repo:openai/codex")`
         returns `pat`; cursors advanced.
- [ ] Second scenario:
      1. Bind `github:org:openai` (fake client returns 2 repos:
         `codex`, `tinker`).
      2. Assert 2 child feeds registered, both with
         `github/repo-mirror` template.
      3. Attempt `github:repo:openai/codex` bind → rejected.
- [ ] Third scenario:
      1. Bind `github:repo:openai/codex` first.
      2. Then bind `github:org:openai` → assert the codex
         child feed exists, and the original
         `github:repo:openai/codex` binding is gone (superseded).
- [ ] No real network — entire test runs against a `FakeGithub`
      that implements `GithubFeedClient`.

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
