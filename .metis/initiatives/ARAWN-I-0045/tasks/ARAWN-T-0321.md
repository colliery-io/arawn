---
id: feed-template-github-review-queue
level: task
title: "Feed template — github/review-queue"
short_code: "ARAWN-T-0321"
created_at: 2026-05-18T12:15:11.992922+00:00
updated_at: 2026-05-18T12:15:11.992922+00:00
parent: ARAWN-I-0045
blocked_by: [ARAWN-T-0320]
effort: S
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

- [ ] New template
      `crates/arawn-feeds/src/templates/github/review_queue.rs`.
- [ ] One query per tick: `is:open is:pr review-requested:@me`.
- [ ] Translates to `GithubReviewRequest` DTOs. `draft` flag
      preserved from the API response so the morning brief
      can deprioritise drafts.
- [ ] Cursor + cadence + rate-limit handling identical to
      [[ARAWN-T-0319]] / [[ARAWN-T-0320]].
- [ ] Smoke + discovery tests.

## Implementation Notes

### Technical Approach

- Same shape as [[ARAWN-T-0320]] but single-query. Should be
  ~50 LOC after the common machinery is in place.

### Dependencies

- Blocked by [[ARAWN-T-0320]] per the user's serial order.

### Risk Considerations

- None substantial — this is the smallest of the three feed
  templates.
