---
id: feed-template-github-notifications
level: task
title: "Feed template — github/notifications"
short_code: "ARAWN-T-0319"
created_at: 2026-05-18T12:15:09.992922+00:00
updated_at: 2026-05-18T13:03:20.708895+00:00
parent: ARAWN-I-0045
blocked_by: [ARAWN-T-0318]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0045
---

# Feed template — github/notifications

## Parent Initiative

[[ARAWN-I-0045]]

## Objective

First user-visible signal stream. Polls GitHub
`/notifications` on the cron schedule, writes typed rows into
`github_notifications`. Drives morning-brief surfacing through
the existing feed runtime.

## Acceptance Criteria

## Acceptance Criteria

- [x] New template
      `crates/arawn-feeds/src/templates/github/notifications.rs`
      registered in `default_registry`.
- [x] Paginated GET `/notifications` with `since` cursor (carried
      in the template's own `CursorState.latest_updated_iso`).
      `Link: rel="next"` followed when `all=true`. The cursor
      replaces `If-Modified-Since`; both approaches solve the
      same problem, GitHub's `since` is the more reliable.
- [x] Translates response → `GithubNotificationProjection` DTOs
      via [[ARAWN-T-0318]]'s `from_notification_json`; raw JSON
      is also persisted to `<feed_dir>/notifications/<id>.json`
      so the dispatch pass can re-project on demand.
- [x] Dispatch arm added in `arawn-projections::dispatch` for
      the `github` provider — currently just handles
      notifications; T-0320/T-0321 will extend to the other two
      subdirs.
- [x] Rate-limit handling: the GitHub client (T-0317) sets
      `Accept` + `X-GitHub-Api-Version` headers; the feed
      client surfaces non-2xx status with body. Backoff on 429
      arrives in a follow-up if hit in practice — the 30-min
      cadence keeps us well under the primary budget (5000/h
      authenticated) at typical personal volume.
- [x] Default cadence 30 min via `defaults()`. Per-feed
      override via `[feeds.<id>.cadence]` in arawn.toml.
- [x] 9 new tests: 6 template-state tests (cursor advance,
      empty batch, auth error, file write, defaults, validate),
      3 client tests (Link-header parsing), plus 2 walk-tests
      in projections. All existing FeedClients impls (10 test
      files) extended with `github()` stubs.

## Status Updates

### 2026-05-18 — feed live

- `GithubFeedClient` trait + `RealGithubClient` adapter in
  `arawn-feeds::clients::github` — single method
  `list_notifications(since, per_page, all) -> Vec<Value>`.
  Follows `Link: rel="next"` headers when `all=true`.
- `FeedClients` trait grew `github()` accessor; `RealClients`
  picks up a `with_github` builder; `NoopClients` returns None.
- Template at `arawn-feeds::templates::github::notifications`.
  Writes raw notifications to `<feed_dir>/notifications/<id>.json`
  with atomic-rename writes (matches gmail pattern). Advances
  `latest_updated_iso` to the max `updated_at` across the batch.
- `arawn-projections::github::walk_notifications_dir` parses
  the on-disk mirror; dispatch.rs gained a `github` arm that
  routes through `dedup_and_write_single_type`.
- Wired main.rs: `RealClients` now calls `with_github` when
  the integration is connected.
- Workspace 1845/0 (1834 → 1845, +11).

## Implementation Notes

### Technical Approach

- Mirrors `arawn-feeds::templates::jira::project_tracker` for
  the cursor + write loop and
  `arawn-feeds::templates::slack::channel_archive` for the
  cron-driven shape.
- The shared HTTP client lives in
  `arawn-integrations::github::client` (T-0317) so rate-limit
  policy is consistent across the three templates.

### Dependencies

- Blocked by [[ARAWN-T-0318]] (tables must exist).
- Blocks [[ARAWN-T-0320]] only in the serial order the user
  requested; the templates are independent in principle.

### Risk Considerations

- Notification volume can be high for active users — early
  testing should sanity-check that `since` cursor + pagination
  keep a single tick under the primary rate-limit budget
  (5000/h authenticated).