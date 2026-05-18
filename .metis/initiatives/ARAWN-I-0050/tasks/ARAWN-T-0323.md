---
id: githubfeedclient-6-new-repo-org
level: task
title: "GithubFeedClient — 6 new repo/org methods"
short_code: "ARAWN-T-0323"
created_at: 2026-05-18T14:33:59.369539+00:00
updated_at: 2026-05-18T14:33:59.369539+00:00
parent: ARAWN-I-0050
blocked_by: []
effort: M
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

- [ ] Six new methods on `GithubFeedClient`:
      - `list_repo_commits(owner, repo, since, max_pages)` — GET
        `/repos/{owner}/{repo}/commits?since=...`.
      - `list_repo_issues(owner, repo, state, since, max_pages)`
        — GET `/repos/{owner}/{repo}/issues?state=&since=`. PRs
        filtered out client-side via `pull_request` field
        absence (GitHub returns both from `/issues`).
      - `list_repo_prs(owner, repo, state, since, max_pages)` —
        GET `/repos/{owner}/{repo}/pulls?state=&sort=updated`.
      - `list_issue_comments(owner, repo, since, max_pages)` —
        GET `/repos/{owner}/{repo}/issues/comments?since=`.
      - `list_pr_review_comments(owner, repo, since, max_pages)`
        — GET `/repos/{owner}/{repo}/pulls/comments?since=`.
      - `list_org_repos(owner, max_pages)` — GET
        `/orgs/{owner}/repos`.
- [ ] Each method paginates via the existing
      `parse_link_next_path` helper. Capped at `max_pages` to
      stay rate-budget-aware.
- [ ] Returns `Vec<Value>` so the template owns parsing.
- [ ] Unit tests with response fixtures (no network):
      single-page, multi-page via Link, empty body, 4xx error
      surface.
- [ ] `NoopClients` / 10 existing test FeedClients impls extended
      with the new methods (default `Ok(vec![])` stubs).

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
