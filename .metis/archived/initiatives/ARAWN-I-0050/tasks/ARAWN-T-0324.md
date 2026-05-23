---
id: projection-feed-types-dtos-commits
level: task
title: "Projection feed_types + DTOs — commits, repo issues, repo prs, comments"
short_code: "ARAWN-T-0324"
created_at: 2026-05-18T14:34:00.803314+00:00
updated_at: 2026-05-18T15:50:35.060650+00:00
parent: ARAWN-I-0050
blocked_by: [ARAWN-T-0323]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0050
---

# Projection feed_types + DTOs — commits, repo issues, repo prs, comments

## Parent Initiative

[[ARAWN-I-0050]]

## Objective

Four new projection feed_types + Rust DTOs + parser functions +
on-disk walk helpers. Substrate for the `github/repo-mirror`
template.

## Acceptance Criteria

## Acceptance Criteria

- [x] Four new feed_type constants in
      `arawn-projections::github`: `REPO_COMMITS_FEED_TYPE`,
      `REPO_ISSUES_FEED_TYPE`, `REPO_PRS_FEED_TYPE`,
      `ISSUE_OR_PR_COMMENTS_FEED_TYPE`.
- [x] Four typed DTOs, each implementing `Projection`:
      - `GithubRepoCommitProjection`
      - `GithubRepoIssueProjection`
      - `GithubRepoPrProjection`
      - `GithubIssueOrPrCommentProjection` (kind discriminator
        `issue_comment` | `pr_review_comment`, derived from
        which URL field the comment carried).
- [x] Parser functions `from_commit_json`,
      `from_repo_issue_json`, `from_repo_pr_json`,
      `from_comment_json` take `(feed_id, owner, repo, …, v)`
      since /commits and /comments rows don't carry owner+repo
      themselves. Body excerpts capped at `BODY_EXCERPT_MAX`.
- [x] On-disk walks `walk_repo_commits_dir`,
      `walk_repo_issues_dir`, `walk_repo_prs_dir`,
      `walk_issue_or_pr_comments_dir` — each iterates
      `<feed_dir>/<owner>/<repo>/<kind>/*.json` via the new
      `walk_repo_kind_dir` helper. Skips legacy user-scoped
      subdirs (`notifications`/`issues_and_prs`/`review_queue`)
      defensively so a shared feed_dir doesn't misparse.
- [x] All four feed_types added to `EMBEDDABLE_FEED_TYPES`;
      `embed_pass::known_feed_types` test list updated.
- [x] 7 new unit tests (commit parser, repo issue parser, repo
      PR merged-state, issue/PR comment kind derivation × 2,
      walk-nested-path, walk-skips-legacy-subdirs) + 4 new
      integration tests through `ProjectionStore` (commits,
      issues with FTS, PRs with head/base, comments with kind).

## Status Updates

### 2026-05-18 — substrate landed

- Four new DTOs + parsers + walks appended to the existing
  `arawn-projections::github` module.
- New helper `walk_repo_kind_dir` traverses the nested
  `<owner>/<repo>/<kind>` layout the repo-mirror template
  will write into.
- Embedding pipeline auto-picks-up the new types via the
  updated `EMBEDDABLE_FEED_TYPES` list.
- Workspace 1877/0 (1866 → 1877, +11).

## Implementation Notes

### Technical Approach

- Mirror `arawn-projections::github` module's existing pattern
  from [[ARAWN-T-0318]]. Add the new feed_types alongside
  notifications/issues_and_prs/review_queue.
- The `walk_*` functions all follow `<owner>/<repo>/<kind>/*.json`
  nesting — refactor the existing `walk_simple_dir` helper to
  optionally recurse for the per-owner-per-repo case.

### Dependencies

- Blocked by [[ARAWN-T-0323]] (client must exist; DTOs match
  the API shape it returns).
- Blocks [[ARAWN-T-0325]] (template needs the projection types).

### Risk Considerations

- Schema lock-in: get column shape right. JSON metadata covers
  the typed fields; hot-path columns can be hoisted later if a
  query profile demands.