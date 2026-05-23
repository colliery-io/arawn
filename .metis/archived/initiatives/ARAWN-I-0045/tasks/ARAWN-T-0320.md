---
id: feed-template-github-issues-and-prs
level: task
title: "Feed template — github/issues-and-prs"
short_code: "ARAWN-T-0320"
created_at: 2026-05-18T12:15:10.992922+00:00
updated_at: 2026-05-18T13:10:21.740762+00:00
parent: ARAWN-I-0045
blocked_by: [ARAWN-T-0319]
archived: true

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

- [x] New template
      `crates/arawn-feeds/src/templates/github/issues_and_prs.rs`,
      registered in `default_registry`.
- [x] Three `/search/issues` queries per tick:
      - `is:open assignee:@me`
      - `is:open author:@me`
      - `is:closed assignee:@me closed:>=<30d-ago>` (retro context).
- [x] `GithubFeedClient::search_issues(query, per_page, max_pages)`
      added. Paginates via `Link: rel="next"`, capped at 5 pages
      (500 results) per query to stay under the 30/min secondary
      rate limit even on heavy installations.
- [x] On disk: `<feed_dir>/issues_and_prs/<owner>__<repo>__<number>.json`.
      Same row can appear in multiple queries; path-keyed dedupe
      avoids double-writes inside a single tick.
- [x] Translates via `from_issue_or_pr_json` (T-0318) — `kind`
      discriminator (`issue`/`pr`) auto-derived from the presence
      of `pull_request` on the row; body excerpt already capped
      at 1KB by the projection layer.
- [x] No cursor — the queries are bounded (`is:open` + 30-day
      closed window) and the projection store de-dupes by
      `source_id` via body-hash on rewrite.
- [x] Default cadence 30 min, overridable per feed.
- [x] 4 new tests: 3-query round-trip with cross-query dedupe,
      empty-results status, default cadence, URL→path parser.
      Plus a dispatch-arm extension in `arawn-projections` so
      issues_and_prs files flow into the
      `github_issues_and_prs` projection table.

## Status Updates

### 2026-05-18 — feed live

- `GithubFeedClient` gained `search_issues(query, per_page,
  max_pages)`. Uses `urlencoding::encode` on the query (URL
  may contain `@`, `>`, `:` etc.) and follows the `Link:
  rel="next"` header until `max_pages` is hit. New crate
  dep: `urlencoding = "2"`.
- Template builds the 3 queries inline (computes the closed
  floor from today − 30d each tick). On query failure, logs
  + skips that query and continues — one broken query
  doesn't poison the others.
- Dedupe across queries via a `HashSet<PathBuf>` of already-
  written paths in the current tick. The projection layer
  also dedupes on `source_id` if rewrites slip through.
- Dispatch arm in `arawn-projections::dispatch` now combines
  notifications + issues_and_prs into one `WriteOutcome`.
  T-0321 will extend with the third arm.
- Workspace 1849/0 (1845 → 1849, +4 new).

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