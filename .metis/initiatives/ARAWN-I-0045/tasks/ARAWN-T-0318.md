---
id: github-projection-schemas-typed
level: task
title: "GitHub projection schemas + typed DTOs"
short_code: "ARAWN-T-0318"
created_at: 2026-05-18T12:15:08.992922+00:00
updated_at: 2026-05-18T12:15:08.992922+00:00
parent: ARAWN-I-0045
blocked_by: [ARAWN-T-0317]
effort: S
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0045
---

# GitHub projection schemas + typed DTOs

## Parent Initiative

[[ARAWN-I-0045]]

## Objective

Add the three GitHub projection tables and their Rust DTOs to
`arawn-projections`. Substrate for the feed templates in
[[ARAWN-T-0319]] / [[ARAWN-T-0320]] / [[ARAWN-T-0321]].

## Acceptance Criteria

- [ ] Refinery migration adds three tables to `projections.db`:
      - `github_notifications` (id, owner, repo, thread_id,
        subject_title, subject_url, reason, kind, unread,
        updated_at, last_read_at).
      - `github_issues_and_prs` (id, owner, repo, number, kind
        (`issue`|`pr`), title, url, state, author, assignees
        JSON, labels JSON, body_excerpt, created_at,
        updated_at, closed_at, merged_at).
      - `github_review_queue` (id, owner, repo, pr_number,
        title, url, author, requested_at, draft).
- [ ] Indexes on (owner, repo, updated_at) for all three;
      (unread) on notifications.
- [ ] Rust DTOs `GithubNotification`, `GithubIssueOrPr`,
      `GithubReviewRequest` with `serde` derives and write
      helpers in `arawn-projections::github`.
- [ ] Round-trip tests for each: insert → query → assert
      field equivalence, including JSON column round-trip.

## Implementation Notes

### Technical Approach

- Pattern-match `arawn-projections::calendar` and
  `arawn-projections::atlassian` for the table+DTO+writer
  shape.
- `body_excerpt` is capped at ~1KB to keep projection rows
  small; the feed runtime can hit the API directly when full
  body is needed downstream.

### Dependencies

- Blocked by [[ARAWN-T-0317]] (token + client must exist; the
  integration health check writes nothing but the schema is
  pre-req for downstream feed templates).
- Blocks [[ARAWN-T-0319]], [[ARAWN-T-0320]], [[ARAWN-T-0321]].

### Risk Considerations

- Schema lock-in: get column shape right now. JSON columns
  for `labels` / `assignees` keep the table extensible without
  another migration.
