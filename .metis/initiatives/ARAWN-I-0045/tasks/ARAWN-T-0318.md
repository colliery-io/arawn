---
id: github-projection-schemas-typed
level: task
title: "GitHub projection schemas + typed DTOs"
short_code: "ARAWN-T-0318"
created_at: 2026-05-18T12:15:08.992922+00:00
updated_at: 2026-05-18T12:46:55.575294+00:00
parent: ARAWN-I-0045
blocked_by: [ARAWN-T-0317]
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

- [x] Three new feed_type constants in `arawn-projections::github`:
      `NOTIFICATIONS_FEED_TYPE = "github_notifications"`,
      `ISSUES_AND_PRS_FEED_TYPE = "github_issues_and_prs"`,
      `REVIEW_QUEUE_FEED_TYPE = "github_review_queue"`. (Schema
      is created on-demand via `ProjectionStore::ensure_feed_type`
      — the projections crate doesn't use refinery migrations
      per-feed-type. The AC's "refinery migration" framing
      misread the architecture; the right shape is the generic
      `ensure_feed_type_tables` pattern that every existing feed
      uses.)
- [x] Indexes — the generic schema gives (source_ts) + (feed_id)
      out of the box. (owner, repo) hot-path filters happen via
      `metadata->>` and are fast enough at our scale; if a
      future profile shows need, hoist to columns then.
- [x] Three Rust DTOs:
      - `GithubNotificationProjection` (id, owner, repo,
        thread_id, subject_title, subject_url, reason, kind,
        unread, updated_at, last_read_at).
      - `GithubIssueOrPrProjection` (id, owner, repo, number,
        kind=`issue`|`pr`, title, url, state, author, assignees
        JSON, labels JSON, body_excerpt (capped at
        `BODY_EXCERPT_MAX`=1024 chars), created_at, updated_at,
        closed_at, merged_at).
      - `GithubReviewRequestProjection` (id, owner, repo,
        pr_number, title, url, author, requested_at, draft).
      Each implements the `Projection` trait, emitting a
      `ProjectionRow` with typed metadata JSON.
- [x] Parser functions `from_notification_json`,
      `from_issue_or_pr_json`, `from_review_request_json` lift
      raw GitHub REST responses into the DTOs. T-0319/T-0320/
      T-0321 feed templates wrap these.
- [x] Round-trip tests:
      - 7 unit tests (DTO parsing, body-excerpt truncation,
        merged-state derivation, URL parsing, defensive
        `None`-on-malformed).
      - 4 integration tests (`tests/github_projections.rs`)
        round-trip each kind through `ProjectionStore`,
        verifying schema creation, FTS indexing, and metadata
        column shape.
- [x] All three feed types added to `EMBEDDABLE_FEED_TYPES` so
      bodies/titles go through the embed pass.

## Status Updates

### 2026-05-18 — substrate landed

- `crates/arawn-projections/src/github.rs` — three Projection
  DTOs + parsers in one file (matches gmail.rs / slack.rs
  single-file convention).
- Embed list updated. Issues/PRs have real bodies worth
  embedding; notifications + review_queue carry titles only but
  still benefit from semantic recall on the morning brief.
- Workspace 1834/0 (1823 → 1834, +11 GitHub projection tests).

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