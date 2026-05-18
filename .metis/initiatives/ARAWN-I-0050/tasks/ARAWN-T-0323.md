---
id: githubfeedclient-6-new-repo-org
level: task
title: "GithubFeedClient — 6 new repo/org methods"
short_code: "ARAWN-T-0323"
created_at: 2026-05-18T14:33:59.369539+00:00
updated_at: 2026-05-18T14:47:07.503324+00:00
parent: ARAWN-I-0050
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0050
---

# GithubFeedClient — 6 new repo/org methods

## Parent Initiative

[[ARAWN-I-0050]]

## Objective

Extend `GithubFeedClient` with the six repo- and org-level REST
calls the `github/repo-mirror` template needs. Substrate only —
no feed template yet.

## Acceptance Criteria

## Acceptance Criteria

- [x] Six new `GithubFeedClient` methods, each paginating via
      the new `RealGithubClient::paginate_array` helper which
      follows `Link: rel="next"` up to `max_pages`:
      - `list_repo_commits` → `/repos/{owner}/{repo}/commits?since=`
      - `list_repo_issues` → `/repos/{owner}/{repo}/issues?state=&since=`
        (then `strip_pr_rows` filters PRs out client-side since
        GitHub returns both from this endpoint).
      - `list_repo_prs` → `/repos/{owner}/{repo}/pulls?state=&sort=updated`
        (since-floor applied client-side via
        `filter_by_updated_at` since `/pulls` doesn't accept
        `since=`).
      - `list_issue_comments` → `/repos/{owner}/{repo}/issues/comments?since=`
      - `list_pr_review_comments` → `/repos/{owner}/{repo}/pulls/comments?since=`
      - `list_org_repos` → `/orgs/{owner}/repos?type=all&sort=updated`
- [x] Returns `Vec<Value>` so the template owns parsing.
- [x] 5 new unit tests for the pure helpers
      (`strip_pr_rows` keeps/drops, `filter_by_updated_at`
      at/after/none/missing). Mocking the real HTTP layer at
      this scope would be heavy infra for marginal coverage;
      the template-layer tests in T-0325 will exercise the
      end-to-end through fakes.
- [x] All three `FakeGithub` test impls (in notifications /
      issues_and_prs / review_queue test modules) extended
      with `Ok(Vec::new())` stubs for the six new methods.
      `NoopClients` returns `None` from `github()` so it
      doesn't need stubs; no other production `GithubFeedClient`
      impls existed.

## Status Updates

### 2026-05-18 — substrate landed

- `RealGithubClient::paginate_array` extracted as a private
  helper — every endpoint that returns a top-level JSON array
  shares the same Link-follow + per-call max-pages cap. The
  existing `list_notifications` retains its own loop because
  it carries the `all` parameter; refactor for free in a
  follow-up if desired.
- `strip_pr_rows` and `filter_by_updated_at` are `pub` free
  functions so they're testable without mocking HTTP.
- Workspace 1866/0 (1861 → 1866, +5).

## Implementation Notes

### Technical Approach

- Mirrors `list_notifications`'s shape from T-0319 — single GET
  + Link-header pagination loop.
- Path construction inline; `state` arg lifted into the query
  string via `urlencoding::encode` (defensive future-proof).

### Dependencies

- Foundation for [[ARAWN-T-0324]] (projection schemas) and the
  `repo-mirror` template in [[ARAWN-T-0325]].

### Risk Considerations

- Search rate limit (30/min) doesn't apply — these are core REST
  endpoints with the 5000/h primary budget.
- Mass-bind of a large org spikes the budget at registration time
  (one `list_org_repos` + many per-repo polls on the next tick).
  [[ARAWN-T-0327]] handles staggering.