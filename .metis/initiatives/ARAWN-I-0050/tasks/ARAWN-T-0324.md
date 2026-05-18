---
id: projection-feed-types-dtos-commits
level: task
title: "Projection feed_types + DTOs — commits, repo issues, repo prs, comments"
short_code: "ARAWN-T-0324"
created_at: 2026-05-18T14:34:00.803314+00:00
updated_at: 2026-05-18T14:34:00.803314+00:00
parent: ARAWN-I-0050
blocked_by: [ARAWN-T-0323]
effort: M
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

- [ ] Four new feed_type constants in
      `arawn-projections::github`:
      - `REPO_COMMITS_FEED_TYPE = "github_repo_commits"`
      - `REPO_ISSUES_FEED_TYPE = "github_repo_issues"`
      - `REPO_PRS_FEED_TYPE = "github_repo_prs"`
      - `ISSUE_OR_PR_COMMENTS_FEED_TYPE = "github_issue_or_pr_comments"`
- [ ] Typed DTOs:
      - `GithubRepoCommitProjection` — sha, message, author,
        parents[], html_url, committed_at.
      - `GithubRepoIssueProjection` — number, title, state,
        labels[], body excerpt, author, assignees[], created_at,
        updated_at, closed_at.
      - `GithubRepoPrProjection` — number, title, state
        (open/closed/merged), labels[], body excerpt, author,
        head/base ref, requested_reviewers[], draft, merged_at.
      - `GithubIssueOrPrCommentProjection` — parent_number,
        body, author, html_url, kind (`issue_comment` |
        `pr_review_comment`), created_at, updated_at.
- [ ] Each DTO implements the `Projection` trait — emits
      `ProjectionRow` with typed metadata JSON.
- [ ] Parser functions `from_commit_json`,
      `from_repo_issue_json`, `from_repo_pr_json`,
      `from_comment_json` lift raw REST responses; body excerpts
      capped at `BODY_EXCERPT_MAX` (existing 1KB constant).
- [ ] On-disk walks: `walk_commits_dir`, `walk_repo_issues_dir`,
      `walk_repo_prs_dir`, `walk_comments_dir` rooted at
      `<feed_dir>/<owner>/<repo>/<kind>/*.json`.
- [ ] All four feed_types added to `EMBEDDABLE_FEED_TYPES` (the
      `embed_pass::known_feed_types` test list updated to match).
- [ ] Round-trip tests through `ProjectionStore` for each kind
      (write → count → fts_search hit on body content for the
      ones with real bodies).

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
