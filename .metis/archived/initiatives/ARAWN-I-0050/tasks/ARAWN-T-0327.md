---
id: org-expand-at-register-list-org
level: task
title: "Org-expand-at-register — list_org_repos + N per-repo feeds"
short_code: "ARAWN-T-0327"
created_at: 2026-05-18T14:34:05.306938+00:00
updated_at: 2026-05-18T18:20:06.581504+00:00
parent: ARAWN-I-0050
blocked_by: [ARAWN-T-0326]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0050
---

# Org-expand-at-register — list_org_repos + N per-repo feeds

## Parent Initiative

[[ARAWN-I-0050]]

## Objective

Turn one `github:org:owner` bind into N
`github/repo-mirror` feed registrations at register-time so
cron is stable and rate-limit-aware.

## Acceptance Criteria

## Acceptance Criteria

- [x] Org bind triggers expand inside main.rs's
      `ExtractorBindHook` — when `parse_github_scope(feed_id)`
      returns `Org { owner }`, the hook spawns an async task
      that runs `expand_github_org`.
- [x] `expand_github_org` calls `RealGithubClient::list_org_repos`
      and `INSERT OR IGNORE INTO feeds` for each repo (template
      `github/repo-mirror`, cadence `*/30 * * * *`, params
      `{owner, name}`). Idempotent on re-bind. On API failure,
      logs warn + bails without failing the bind.
- [x] Late-bound cell
      `Arc<RwLock<Option<Arc<GithubIntegration>>>>` declared
      before the bind tool, populated after the github init
      block. Empty cell = no-op expand with a debug log.
- [x] Org unbind: `WorkstreamUnbindTool` runs
      `DELETE FROM feeds WHERE id LIKE 'github-repo:owner/%'`
      so every child feed registered via the expand goes away.
- [-] Cron-offset staggering: deferred. All registered repos
      currently share `*/30 * * * *`. Filed as a known follow-up
      — hash-based offset rewrite when bunching becomes a real
      cost.
- [x] One new unit test
      (`unbind_org_scope_sweeps_all_child_feeds`) covering the
      sweep behaviour. The `list_org_repos`-driven expansion
      itself is exercised in [[ARAWN-T-0328]]'s smoke test (no
      real network needed thanks to the GithubFeedClient fake).

## Status Updates

### 2026-05-18 — expand wired

- `expand_github_org` is a file-scope `async fn` in main.rs so
  it can use the binary's access to `RealGithubClient` and the
  shared `Store` without dragging arawn-engine into the dep
  graph.
- Late-bound cell threads `GithubIntegration` from the github
  init (later in main) back into the bind hook (created
  earlier).
- Workspace 1892/0 (1891 → 1892, +1).

## Implementation Notes

### Technical Approach

- New Store field on the org binding payload: list of child
  `feed_id`s. Or maintain a separate table
  `github_org_child_feeds(workstream, owner, feed_id)`. The
  latter is cleaner since the bindings array stays just strings.
- Cron offset: deterministic from `hash(feed_id) % 30`. Same
  shape every restart so the schedule doesn't churn.

### Dependencies

- Blocked by [[ARAWN-T-0326]] (org bind path runs before this
  expand fires).

### Risk Considerations

- Large org (100s of repos) spikes registration time. Either
  cap (skip if > 200 repos and emit a warning) or stream the
  expand asynchronously. Decide during implementation.