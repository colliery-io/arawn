---
id: unit-tests-for-arawn-plugin
level: task
title: "Unit tests for arawn-plugin/subscription.rs (61.5% coverage)"
short_code: "ARAWN-T-0309"
created_at: 2026-03-09T15:43:32.356869+00:00
updated_at: 2026-03-10T00:55:56.707751+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Unit tests for arawn-plugin/subscription.rs (61.5% coverage)

## Objective

Improve unit test coverage for `arawn-plugin/src/subscription.rs` from 61.5% to 80%+. This module handles plugin event subscriptions (hooks, notifications) and has runtime behavior that's difficult to exercise through E2E tests (0% E2E coverage). The uncovered paths likely include error handling, subscription lifecycle, and edge cases in event dispatch.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P3 - Low (when time permits)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Unit tests for subscription creation and teardown
- [ ] Unit tests for event dispatch to multiple subscribers
- [ ] Unit tests for subscription filtering/matching
- [ ] Error path coverage (invalid subscriptions, dispatch failures)
- [ ] Coverage for `subscription.rs` reaches 80%+

## Implementation Notes

### Key Files
- `crates/arawn-plugin/src/subscription.rs` (472/767 lines, 61.5% unit; 0% E2E)
- May need to review what patterns are untested by reading the file

## Status Updates

### Session 1 - COMPLETED
- **Starting coverage**: subscription.rs 61.54%
- **Final coverage**: subscription.rs **92.15%** (target was 80%+)
- Added ~45 new tests (63 total, 0 ignored) covering:
  - RuntimePluginsConfig: remove_subscription, add_duplicate, is_enabled, invalid JSON, save creates parent dirs
  - SubscriptionManager: no_project_dir, accessors (read/mut), add global/project subscriptions, set global/project enabled, project overrides global, enabled falls through to subscription flag, global enabled filter, save global/project config, save with no project, cache_dir_for, plugin_dirs, all_subscriptions priority
  - GitOps against octocat/Hello-World: clone, pull, clone over non-git dir, invalid URL error, invalid ref error, current_commit, current_branch
  - sync_subscription: clone + update flow against real repo
  - sync_all: mixed local + remote subscriptions
  - plugin_dir_for: remote synced vs unsynced
  - sync_all_async: real repo clone, no clone URL, empty, local skipped
- All 63 tests pass, clippy clean, fmt clean