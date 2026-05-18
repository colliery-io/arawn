---
id: github-repo-mirror-feed-template
level: task
title: "github/repo-mirror feed template"
short_code: "ARAWN-T-0325"
created_at: 2026-05-18T14:34:02.312795+00:00
updated_at: 2026-05-18T16:35:57.723629+00:00
parent: ARAWN-I-0050
blocked_by: [ARAWN-T-0324]
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

- [x] New template `crates/arawn-feeds/src/templates/github/repo_mirror.rs`,
      registered in `default_registry`.
- [x] Required params `owner` + `name`; 30-min default cadence.
- [x] `CursorState` with four per-kind RFC3339 cursors (commits/
      issues/prs/comments). String compare advances monotonically;
      empty batch preserves the prior cursor.
- [x] Each tick hits all four `GithubFeedClient` calls.
      Comments fetched from /issues/comments + /pulls/comments
      and merged into one on-disk dir — the projection parser
      derives `kind` from URL fields. Writes atomic (tmp + rename)
      to `<feed_dir>/<owner>/<name>/<kind>/<id>.json`.
- [x] Dispatch arm in `arawn-projections::dispatch` extended:
      the existing `github` arm now walks the four new
      `<owner>/<repo>/<kind>` subdirs into the matching tables.
- [x] Per-kind partial-failure tolerance: one kind 4xx logs +
      skips that batch; other kinds still write and advance.
- [x] 8 unit tests covering validate / defaults / full write-and-
      advance / partial-failure / empty-preserves-cursor /
      since-floor pass-through / missing-clients-auth-error /
      pure `advance` helper.

## Status Updates

### 2026-05-18 — template live

- One template, two-level write loop (kind × row). Failed kind
  fetches downgrade to `warn!` + empty batch so a single 4xx
  doesn't poison the whole tick.
- Workspace 1885/0 (1877 → 1885, +8).

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