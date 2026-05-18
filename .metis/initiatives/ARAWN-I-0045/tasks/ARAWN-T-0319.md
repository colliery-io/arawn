---
id: feed-template-github-notifications
level: task
title: "Feed template — github/notifications"
short_code: "ARAWN-T-0319"
created_at: 2026-05-18T12:15:09.992922+00:00
updated_at: 2026-05-18T12:15:09.992922+00:00
parent: ARAWN-I-0045
blocked_by: [ARAWN-T-0318]
effort: M
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

- [ ] New template under
      `crates/arawn-feeds/src/templates/github/notifications.rs`
      registered in the template registry.
- [ ] Paginated GET `/notifications` with `since` cursor + an
      `If-Modified-Since` header pulled from the
      `ExtractorCursorStore` for `(workstream, "github_notifications")`.
- [ ] Translates the API response into `GithubNotification`
      DTOs and writes via the T-0318 store; cursor advances
      monotonically on success.
- [ ] Rate-limit handling: respects `X-RateLimit-Remaining` /
      `X-RateLimit-Reset` headers; on a 429 with
      `Retry-After`, sleeps the indicated interval before
      retrying.
- [ ] Default cadence 30 min, user-overridable via
      `[feeds.github_notifications.cadence]` in arawn.toml.
- [ ] Discovery test + smoke test mirrors the
      `slack/channel_archive` / `gmail/inbox_archive` pattern.

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
