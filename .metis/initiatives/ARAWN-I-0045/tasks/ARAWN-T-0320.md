---
id: feed-template-github-issues-and-prs
level: task
title: "Feed template — github/issues-and-prs"
short_code: "ARAWN-T-0320"
created_at: 2026-05-18T12:15:10.992922+00:00
updated_at: 2026-05-18T12:15:10.992922+00:00
parent: ARAWN-I-0045
blocked_by: [ARAWN-T-0319]
effort: M
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0045
---

# Feed template — github/issues-and-prs

## Parent Initiative

[[ARAWN-I-0045]]

## Objective

User-assigned + user-authored issues and PRs across the
installation scope. Drives "what's on my plate" surfacing in
the morning brief and weekly priorities.

## Acceptance Criteria

- [ ] New template
      `crates/arawn-feeds/src/templates/github/issues_and_prs.rs`.
- [ ] Two search queries per tick:
      - `is:open assignee:@me`
      - `is:open author:@me`
      Plus a third for recently-closed (last 30d) so retro
      gather has context: `is:closed assignee:@me closed:>=<30d-ago>`.
- [ ] Translates to `GithubIssueOrPr` DTOs with `kind`
      discriminator (issue / pr). Body excerpt capped at 1KB.
- [ ] Cursor: per-query `since` stored in
      `ExtractorCursorStore`. Monotonic advance on success.
- [ ] Rate-limit handling shared with [[ARAWN-T-0319]] via the
      common client.
- [ ] Default cadence 30 min, overridable per feed.
- [ ] Smoke test against fixture responses; discovery test
      asserts the template registers.

## Implementation Notes

### Technical Approach

- Use the `/search/issues` endpoint (capped at 1000 results
  per query — fine for a personal scope; assert behaviour
  documented if a user blows past it).
- Pagination: GitHub returns `Link: <...>; rel="next"`; the
  client's pagination helper from T-0317 should handle this
  the same way it does for `/notifications`.

### Dependencies

- Blocked by [[ARAWN-T-0319]] per the user's serial-order
  preference. Technically only depends on T-0317 / T-0318.

### Risk Considerations

- `/search/issues` has a stricter secondary rate limit (30
  requests / minute). The three queries × number of repos
  should stay well under this; document the boundary.
