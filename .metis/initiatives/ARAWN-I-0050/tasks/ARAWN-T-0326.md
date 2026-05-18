---
id: bind-tool-org-supersedes-repo
level: task
title: "Bind tool — org-supersedes-repo + scope-bind registers feed"
short_code: "ARAWN-T-0326"
created_at: 2026-05-18T14:34:03.819733+00:00
updated_at: 2026-05-18T14:34:03.819733+00:00
parent: ARAWN-I-0050
blocked_by: [ARAWN-T-0325]
effort: M
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0050
---

# Bind tool — org-supersedes-repo + scope-bind registers feed

## Parent Initiative

[[ARAWN-I-0050]]

## Objective

Make `WorkstreamBindTool`'s github scope strings actually do
something. Repo bind registers a `github/repo-mirror` feed; org
bind triggers [[ARAWN-T-0327]]'s expand. Both enforce the
"org supersedes repo" rule.

## Acceptance Criteria

- [ ] `WorkstreamBindTool::execute` for `github:repo:owner/name`:
      - If `github:org:owner` is already bound on ANY workstream
        → reject with: `"covered by github:org:owner bound to
        <ws-name>"`.
      - Otherwise: persist the binding, register a feed
        `github-repo:owner/name` running `github/repo-mirror`
        with params `{owner, name}`, fire bind-backfill hook.
- [ ] `WorkstreamBindTool::execute` for `github:org:owner`:
      - Find all `github:repo:owner/*` bindings (any workstream).
      - For each: drop the binding, unregister the corresponding
        feed (`github-repo:owner/<name>`).
      - Emit a `ServerNotice` listing what was superseded.
      - Persist the org binding, then delegate to [[ARAWN-T-0327]]
        for the per-repo expand + register.
- [ ] Bind-backfill hook for github-repo: feeds is auto-derived
      from `feed_id.starts_with("github-repo:")` — fans out
      backfill on the four new feed_types from [[ARAWN-T-0324]].
- [ ] `WorkstreamUnbindTool` extended to unregister the
      corresponding feed when a scope binding is removed.
- [ ] Tests:
      - Repo bind rejected when org of same owner exists.
      - Org bind sweeps existing repo bindings (across multiple
        workstreams) and unregisters their feeds.
      - Successful repo bind creates a feed with the expected
        id, template, and params.
      - Unbind drops the feed.

## Implementation Notes

### Technical Approach

- Adds a Store method `find_workstream_bindings_starting_with(prefix)`
  returning `Vec<(workstream_name, binding)>` so the org-supersedes
  check is cheap.
- Feed registration goes through `FeedStore::create` with a
  synthetic `feed_id` like `github-repo:owner/name`. Cron schedule
  is the template's default (30 min).

### Dependencies

- Blocked by [[ARAWN-T-0325]] (template must exist to register).
- Pairs with [[ARAWN-T-0327]] which handles the org expand
  inside the same bind-execute path.

### Risk Considerations

- Atomicity: if the org bind sweep partially fails (unregister
  one repo feed, then crash before the org feeds register), we
  could leave the user in a half-state. Wrap the bind transaction
  to roll back any sweep that doesn't reach the register step.
