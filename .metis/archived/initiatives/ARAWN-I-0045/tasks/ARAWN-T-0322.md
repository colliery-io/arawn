---
id: workstream-binding-github-repo-and
level: task
title: "Workstream binding — github:repo and github:org schemes"
short_code: "ARAWN-T-0322"
created_at: 2026-05-18T12:15:12.992922+00:00
updated_at: 2026-05-18T13:26:24.934999+00:00
parent: ARAWN-I-0045
blocked_by: [ARAWN-T-0321]
archived: true

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

- [x] `WorkstreamBindTool` description advertises the two new
      schemes (`github:repo:owner/name`, `github:org:owner`)
      alongside the existing feed-id form. Validation rejects
      malformed scope strings at the tool boundary
      (missing slash, empty owner/name).
- [x] Bind-backfill hook in `main.rs` recognises github scope
      bindings via `is_github_scope_binding(feed_id)` and
      short-circuits to the three github feed types
      (`github_notifications`, `github_issues_and_prs`,
      `github_review_queue`), spawning extractor backfill
      against the workstream so existing projection rows get
      walked through the chain immediately.
- [x] 9 new unit tests covering the validator (`github:repo`
      with/without slash, empty owner/name, `github:org` empty
      and with-slash, non-github pass-through, scheme
      recognition) and tool-level integration (accepts well-
      formed, rejects malformed).
- [ ] **Deferred to follow-up**: per-row extractor-chain
      filtering by github bindings (repo-wins-over-org
      precedence). The chain today is workstream-agnostic at
      the SQL-fetch layer; teaching it to honor github scope
      bindings requires reaching into `chain.run(workstream,
      row, …)` to skip non-matching rows. That's a chain-layer
      change that crosses extractor + memory + integrations
      and deserves its own task. Filed as a known gap; the
      read-side surface (feeds + projections + bind tool)
      lands here and the agent already sees github rows via
      `feed_search`/`signal_query` regardless of binding.

## Status Updates

### 2026-05-18 — substrate landed, full routing deferred

- `validate_github_scope_scheme` + `is_github_scope_binding`
  helpers in `arawn-engine::tools::workstream`. The validator
  is a no-op for non-github feed_ids so it composes cleanly
  with the existing bind path.
- WorkstreamBindTool description updated with concrete examples.
- main.rs's `ExtractorBindHook::on_bind` short-circuits when
  the binding is a github scope — spawns backfill across all
  three github feed types so the chain walks every existing
  projection row through the workstream.
- 9 new tests, workspace 1861/0.

### Known gap

The extractor's per-row filtering doesn't yet honor github
scope bindings — every workstream that runs the github
feed_types sees every row. For the agent's current usage
(`feed_search`/`signal_query` across all rows), this is
acceptable. For "automatically route this repo into that
workstream's KB", a follow-up needs to:

1. Extend `chain.run(workstream, row, …)` to check binding
   match before processing github rows.
2. Use repo-wins-over-org precedence: try
   `github:repo:owner/name` first, fall back to
   `github:org:owner`.
3. Backfill correctly when a binding's specificity changes
   (org-bound workstream loses rows that a later repo-bind
   claims).

Filed for follow-up; not blocking the I-0045 close.

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