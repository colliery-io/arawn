---
id: org-expand-at-register-list-org
level: task
title: "Org-expand-at-register — list_org_repos + N per-repo feeds"
short_code: "ARAWN-T-0327"
created_at: 2026-05-18T14:34:05.306938+00:00
updated_at: 2026-05-18T14:34:05.306938+00:00
parent: ARAWN-I-0050
blocked_by: [ARAWN-T-0326]
effort: S
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0050
---

# Org-expand-at-register — list_org_repos + N per-repo feeds

## Parent Initiative

[[ARAWN-I-0050]]

## Objective

Turn one `github:org:owner` bind into N
`github/repo-mirror` feed registrations at register-time so
cron is stable and rate-limit-aware.

## Acceptance Criteria

- [ ] On successful org bind (from [[ARAWN-T-0326]]):
      - Call `GithubFeedClient::list_org_repos(owner)`.
      - For each repo, register a feed
        `github-repo:owner/name` running `github/repo-mirror`
        with params `{owner, name}`. Idempotent — re-binding
        the same org skips repos that already have a feed.
      - Persist a record of which feed_ids belong to the org
        binding so unbind can find them.
- [ ] If `list_org_repos` fails or returns empty, the org bind
      still succeeds — emit a `ServerNotice` indicating no
      repos were found / fetch failed, and let the user retry
      after fixing the App scope.
- [ ] Stagger cron offsets across the N repos so all polls
      don't fire on the same minute boundary (uses the existing
      feed-cadence offset logic if available; otherwise hash
      `feed_id` → 0..29 min offset).
- [ ] Unbind of an org binding tears down all child
      `github-repo:` feeds it registered.
- [ ] Tests:
      - Org bind with 3 repos creates 3 feeds.
      - Re-binding the same org is idempotent (no duplicate
        feeds).
      - Empty `list_org_repos` doesn't error.
      - Unbind sweeps all 3 child feeds.

## Implementation Notes

### Technical Approach

- New Store field on the org binding payload: list of child
  `feed_id`s. Or maintain a separate table
  `github_org_child_feeds(workstream, owner, feed_id)`. The
  latter is cleaner since the bindings array stays just strings.
- Cron offset: deterministic from `hash(feed_id) % 30`. Same
  shape every restart so the schedule doesn't churn.

### Dependencies

- Blocked by [[ARAWN-T-0326]] (org bind path runs before this
  expand fires).

### Risk Considerations

- Large org (100s of repos) spikes registration time. Either
  cap (skip if > 200 repos and emit a warning) or stream the
  expand asynchronously. Decide during implementation.
