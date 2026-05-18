---
id: github-workstream-scoped-scraper
level: initiative
title: "GitHub workstream-scoped scraper — per-repo + per-org polling"
short_code: "ARAWN-I-0050"
created_at: 2026-05-18T14:25:46.761720+00:00
updated_at: 2026-05-18T14:28:30.418558+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/decompose"


exit_criteria_met: false
estimated_complexity: M
initiative_id: github-workstream-scoped-scraper
---

# GitHub workstream-scoped scraper — per-repo + per-org polling

## Context

I-0045 shipped three **user-scoped** GitHub feeds (notifications,
issues-and-prs, review-queue) for the morning-brief surface, plus a
`WorkstreamBindTool` that accepts `github:repo:owner/name` and
`github:org:owner` scope strings. The bind side stores the strings
but nothing in the runtime acts on them — there's no scraper that
ever fires "all commits + issues + PRs + comments under
`openai/codex`" into the workstream's KB.

That's the gap. This initiative builds the scraper.

## Goals & Non-Goals

**Goals:**

- Repo binding (`github:repo:openai/codex`) registers a scheduled
  poll that, every 30 min, pulls **commits, issues, PRs, and their
  comment threads** for that repo into projection tables. Cursors
  per kind track "what we've seen."
- Org binding (`github:org:openai`) does the same for **every
  viewable repo under that org**. On register, expands to one
  scheduled poll per repo (so cron is predictable and rate-budget
  scales linearly with repo count).
- Org **supersedes** repo:
  - Bind repo when org is already bound → reject with a clear
    "covered by `github:org:openai`" message.
  - Bind org when repos are already bound under that owner →
    silently drop the repo schedules (and their bindings).
- Storage layout: `<owner>/<repo>/<kind>/<id>.json` so repo-scope
  and org-scope writes share paths. Idempotent at the filesystem
  layer — a stray double-schedule doesn't corrupt anything.
- Bind-time backfill: when a binding lands, walk existing data
  for that repo/org into the workstream's KB.

**Non-Goals:**

- The three user-scoped feeds from I-0045. They stay — separate
  purpose (the user's morning attention stream, regardless of
  any workstream binding).
- Write tools (comment, label, mark-read). Still out per I-0045's
  read-only decision.
- Per-user-comment threading expansion across PRs that touch many
  repos. Comments stay on their parent issue/PR row.
- Org admin operations (member management, audit logs).

## Design Decisions (locked 2026-05-18)

- **Auth + integration substrate**: reuses [[ARAWN-I-0045]]'s
  GitHub App + `GithubIntegration` + `GithubClient`. No new auth
  surface.
- **Org-expand-at-register**: when `github:org:openai` is bound,
  the bind hook calls `list_org_repos` once and registers one
  `github/repo-mirror` feed per repo. This keeps cron stable and
  rate-limit-aware; the alternative (one feed that re-fans-out
  every tick) blows the secondary search budget.
- **Org supersedes repo**: enforced at both the bind tool (reject
  redundant repo bind when org exists) and the org-bind hook
  (drop existing repo schedules + bindings under the org).
- **One template, parameterised**: `github/repo-mirror` with
  required `owner` + `name` params handles all the per-repo work.
  Org binding is a registration-time fan-out into N
  `repo-mirror` feeds, not a separate runtime template.
- **Cursors are per-(feed_id, kind)**: commits / issues / PRs /
  comments each get their own monotonic `since` cursor in the
  feed's `CursorState` JSON.
- **Pre-existing rows on supersession**: when org-bind drops a
  repo schedule, leave the projection rows in place — the
  storage path is identical so the org's scrape will overwrite/
  update them naturally on the next tick.

## Detailed Design

### Storage layout

Inside each feed_dir (one per binding):
```
<feed_dir>/
    openai/codex/
        commits/<sha>.json
        issues/<number>.json
        prs/<number>.json
        comments/<owner>/<repo>/<parent_number>/<comment_id>.json
```

### Projection tables

Four new feed_types — same generic `ensure_feed_type_tables`
shape used by everything else, typed `metadata` JSON:

- `github_repo_commits` — sha, message, author, parents, html_url.
- `github_repo_issues` — number, title, state, labels[], body
  excerpt, author, assignees[]. (Distinct from
  `github_issues_and_prs` which is user-search-scoped.)
- `github_repo_prs` — number, title, state (open/closed/merged),
  labels[], body excerpt, head/base ref, author, requested
  reviewers, draft.
- `github_issue_or_pr_comments` — parent_number, body, author,
  url. Both issue comments and PR review comments land here with
  a `kind` discriminator in metadata.

### GithubFeedClient additions

```rust
list_repo_commits(owner, repo, since: Option<DateTime<Utc>>)
list_repo_issues(owner, repo, state: &str, since: Option<DateTime<Utc>>)
list_repo_prs(owner, repo, state: &str, since: Option<DateTime<Utc>>)
list_issue_comments(owner, repo, since: Option<DateTime<Utc>>)
list_pr_review_comments(owner, repo, since: Option<DateTime<Utc>>)
list_org_repos(owner) -> Vec<RepoMeta>  // for org-expand-at-register
```

### Bind tool

Two new behaviours:

- **Repo bind** with `github:org:<owner>` already bound (any
  workstream) → return error: "covered by github:org:<owner>
  bound to <ws-name>".
- **Org bind** with `github:repo:<owner>/<repo>` already bound
  (any workstream) → drop those bindings + unregister their
  feeds, then proceed with org bind. Emit a notice listing what
  was superseded.
- After any successful scope bind, register the feed(s) and fire
  the existing bind-backfill hook so existing projection rows
  walk through the chain.

### Dispatch + routing

The bind hook registers feeds, so each repo gets its own feed_id
(e.g. `github-repo:openai/codex`). The standard
`find_workstream_for_feed` lookup then routes rows correctly
without any per-row chain filter — the workstream binding is
back to being feed-id-shaped, not metadata-shaped.

## Alternatives Considered

- **Per-row chain filter** (what I almost did mid-I-0045). Stores
  scope strings on the workstream, then filters every github row
  at extraction time. Rejected because it's a global poll that
  blows API budget on big installations and conflates the
  workstream's *scope* with the agent's user-level signals.
- **Org as runtime fan-out** instead of register-time. Rejected
  because each tick would have to list_org_repos + iterate; with
  big orgs (100s of repos) this blows the search rate limit.
- **Keep repo-binds alive when org binds** (parallel polls).
  Rejected per the user's "shouldn't ever fire" call —
  redundant API spend.

## Implementation Plan

Decompose during design phase. Rough shape (~6 tasks):

1. `GithubFeedClient` additions — six new methods + tests.
2. Four new projection feed_types + DTOs + parsers + walks.
3. `github/repo-mirror` template (per-kind cursors + storage layout).
4. Bind tool: org-supersedes-repo logic + scope-bind → feed-register.
5. Org-expand-at-register: list_org_repos + register N feeds per
   org binding.
6. Bind-backfill + dispatch arm extension for the new feed_types.

## Exit Criteria

- `/workstream bind <ws> github:repo:openai/codex` registers a
  feed, schedules a 30-min poll, and one tick later writes
  commits/issues/PRs/comments to projection tables.
- `/workstream bind <ws> github:org:openai` lists the org's repos
  at register-time and schedules N per-repo polls.
- `github:repo:` bind while `github:org:` exists rejects with a
  clear message.
- `github:org:` bind while `github:repo:` exists drops those
  schedules + bindings (idempotent path-level overlap means the
  org's scrape picks up where the repo's left off).
- Workstream KB reflects only rows from bound repos/orgs after
  the next extractor tick.
- Existing I-0045 user-scoped feeds (notifications, issues-and-prs,
  review-queue) continue to work unchanged.