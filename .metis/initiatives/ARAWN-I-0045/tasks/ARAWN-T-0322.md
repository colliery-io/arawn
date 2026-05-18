---
id: workstream-binding-github-repo-and
level: task
title: "Workstream binding — github:repo and github:org schemes"
short_code: "ARAWN-T-0322"
created_at: 2026-05-18T12:15:12.992922+00:00
updated_at: 2026-05-18T12:15:12.992922+00:00
parent: ARAWN-I-0045
blocked_by: [ARAWN-T-0321]
effort: M
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0045
---

# Workstream binding — github:repo and github:org schemes

## Parent Initiative

[[ARAWN-I-0045]]

## Objective

Make GitHub projection rows routable into workstreams via
`/workstream bind <ws> github:repo:owner/name` and
`/workstream bind <ws> github:org:owner`. Repo bindings win
over org bindings on conflict.

## Acceptance Criteria

- [ ] `WorkstreamBindTool` accepts the two new binding schemes
      alongside the existing slack / gmail / drive schemes.
- [ ] Bind-backfill hook in `main.rs`: on first bind, pull all
      existing rows from the three github_* tables matching
      the scope and route them into the workstream's KB via
      the extractor's existing pass.
- [ ] Extractor routing pass: for an incoming projection row,
      resolve owner+repo to the most-specific binding first
      (repo > org). Multiple workstreams with the same
      specificity → tie-break on creation order (oldest wins,
      matching the existing routing pass behaviour).
- [ ] `WorkstreamShowTool` lists active github bindings under
      a `github_bindings` section.
- [ ] Unit tests: repo-wins-over-org resolution, bind-backfill
      writes the right rows, extractor route + cursor advance
      round-trip.
- [ ] End-to-end smoke test (gated, real-API): the
      `priority-completion-feedback`-style scenario from
      I-0049, but seeded with synthetic GitHub projection rows
      under `proj-a` workstream → agent sees them via
      `feed_search` or `signal_query`.

## Implementation Notes

### Technical Approach

- Existing binding schemes parse `scheme:rest` in
  `WorkstreamBindTool::parse`. Add two new arms:
  - `github:repo:owner/name` → store `{kind: "github_repo",
     owner, name}` in the binding payload.
  - `github:org:owner` → `{kind: "github_org", owner}`.
- The extractor's per-feed-type routing pass needs a
  github_repo + github_org lookup. Reuse the
  `workstream_bindings` query helper; add an extra resolve
  step that walks repo → org if no repo match.
- Tests: pattern-match
  `crates/arawn-engine/src/tools/workstream.rs` for the bind
  parse tests, and `crates/arawn-extractor/src/runner.rs`
  for the routing tests.

### Dependencies

- Blocked by [[ARAWN-T-0321]] per the user's serial order;
  technically depends on [[ARAWN-T-0319]] / [[ARAWN-T-0320]] /
  [[ARAWN-T-0321]] producing rows to route.

### Risk Considerations

- Routing conflicts: a repo bound to workstream A AND an org
  binding to workstream B. The repo-wins rule is explicit;
  document it in `WorkstreamBindTool`'s description and the
  workstream-show output so users don't get surprised.
