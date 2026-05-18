---
id: github-repo-mirror-feed-template
level: task
title: "github/repo-mirror feed template"
short_code: "ARAWN-T-0325"
created_at: 2026-05-18T14:34:02.312795+00:00
updated_at: 2026-05-18T14:34:02.312795+00:00
parent: ARAWN-I-0050
blocked_by: [ARAWN-T-0324]
effort: M
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0050
---

# github/repo-mirror feed template

## Parent Initiative

[[ARAWN-I-0050]]

## Objective

Single feed template that mirrors one repo's commits, issues,
PRs, and their comment threads. Parametrised on `owner` + `name`
so org-bind's fan-out in [[ARAWN-T-0327]] can register N
instances in one shot.

## Acceptance Criteria

- [ ] New template `crates/arawn-feeds/src/templates/github/repo_mirror.rs`
      registered in `default_registry`.
- [ ] Required params: `owner: string`, `name: string` (both
      validated non-empty).
- [ ] Default cadence 30 min, overridable per feed.
- [ ] `CursorState` with four per-kind monotonic `since` cursors:
      `commits_since`, `issues_since`, `prs_since`,
      `comments_since`. Advances to the max `updated_at` (or
      `committed_at` for commits) across each kind's batch.
- [ ] On each tick, hits all four `GithubFeedClient` calls
      under per-kind rate-aware caps, writes raw JSON to:
      ```
      <feed_dir>/<owner>/<name>/commits/<sha>.json
      <feed_dir>/<owner>/<name>/issues/<number>.json
      <feed_dir>/<owner>/<name>/prs/<number>.json
      <feed_dir>/<owner>/<name>/comments/<comment_id>.json
      ```
- [ ] Dispatch arm in `arawn-projections::dispatch` extended to
      walk all four new kinds and dedup-write into the four new
      projection tables.
- [ ] 6+ unit tests covering: validate, defaults, single-kind
      cursor advance, all-empty `no-new-items` status,
      per-kind partial-failure tolerance (one kind 4xx →
      others still write), file write idempotence on retry.

## Implementation Notes

### Technical Approach

- Mirrors the `issues_and_prs` template's shape from [[ARAWN-T-0320]]
  but with four kinds in one tick instead of three search
  queries. Each kind is independent; a failure on one logs +
  skips, others continue.
- File writes are atomic (tmp + rename) — re-runs idempotent on
  source_id collision.

### Dependencies

- Blocked by [[ARAWN-T-0324]] (projection types + walks).
- Blocks [[ARAWN-T-0326]] (bind tool needs the template name).

### Risk Considerations

- Comment volume per repo can be large. The per-kind
  `max_pages` cap (set in T-0323) keeps a single tick bounded;
  high-churn repos will catch up over several ticks.
