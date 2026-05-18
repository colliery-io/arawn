---
id: feed-template-github-review-queue
level: task
title: "Feed template — github/review-queue"
short_code: "ARAWN-T-0321"
created_at: 2026-05-18T12:15:11.992922+00:00
updated_at: 2026-05-18T13:15:20.856758+00:00
parent: ARAWN-I-0045
blocked_by: [ARAWN-T-0320]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0045
---

# Feed template — github/review-queue

## Parent Initiative

[[ARAWN-I-0045]]

## Objective

PRs the user has been asked to review. Separate stream from
issues-and-prs because review duty is the highest-priority
GitHub signal (someone is blocked on you).

## Acceptance Criteria

- [x] New template
      `crates/arawn-feeds/src/templates/github/review_queue.rs`,
      registered in `default_registry`.
- [x] One search query per tick:
      `is:open is:pr review-requested:@me`. Capped at 3 pages
      (300 results) — review-requested-on-me is naturally
      bounded so this stays well under any rate budget.
- [x] Translates via `from_review_request_json` (T-0318);
      `draft` flag preserved in the projection's metadata for
      the morning brief.
- [x] Cadence 30 min default, overridable per feed.
- [x] Dispatch arm in `arawn-projections` extended to flow
      review_queue rows into the `github_review_queue`
      projection table.
- [x] 3 new tests (write + dedupe, empty-status, default
      cadence). Workspace 1852/0.

## Status Updates

### 2026-05-18 — feed live

- Thin template: single query, no cross-query dedupe needed,
  no cursor (the bounded query handles freshness).
- Reuses `GithubFeedClient::search_issues` from T-0320 — no
  client surface change.
- Rounds out the read-side surface. Together with T-0319
  (notifications) and T-0320 (issues + PRs), all three
  GitHub feeds are now poll-able.

## Implementation Notes

### Technical Approach

- Same shape as [[ARAWN-T-0320]] but single-query. Should be
  ~50 LOC after the common machinery is in place.

### Dependencies

- Blocked by [[ARAWN-T-0320]] per the user's serial order.

### Risk Considerations

- None substantial — this is the smallest of the three feed
  templates.