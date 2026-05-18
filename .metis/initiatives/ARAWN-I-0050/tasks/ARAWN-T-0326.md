---
id: bind-tool-org-supersedes-repo
level: task
title: "Bind tool — org-supersedes-repo + scope-bind registers feed"
short_code: "ARAWN-T-0326"
created_at: 2026-05-18T14:34:03.819733+00:00
updated_at: 2026-05-18T16:54:28.147419+00:00
parent: ARAWN-I-0050
blocked_by: [ARAWN-T-0325]
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

- [x] Repo bind path: parses `github:repo:owner/name`; rejects
      with explicit "covered by `github:org:<owner>` already
      bound to workstream `<ws>`" message when an org binding
      exists. Otherwise persists the binding AND inserts a feed
      record `github-repo:owner/name` (template
      `github/repo-mirror`, cadence `*/30 * * * *`, params
      `{owner, name}`).
- [x] Org bind path: parses `github:org:owner`; finds every
      `github:repo:owner/*` binding across all workstreams,
      removes them, and deletes the matching `github-repo:` feed
      records. Returns the list of superseded bindings in the
      success body so callers can surface them.
- [x] Bind-backfill hook in main.rs's `ExtractorBindHook` now
      walks the four T-0324 feed_types
      (`github_repo_commits` / `github_repo_issues` /
      `github_repo_prs` / `github_issue_or_pr_comments`) in
      addition to the three user-scoped ones when a github
      scope binding lands.
- [x] `WorkstreamUnbindTool` recognises `github:repo:` schemes
      and deletes the matching `github-repo:` feed record.
- [x] Feed-table mutations go through raw SQL on
      `Store::database().conn()` rather than
      `arawn-feeds::FeedStore` to avoid an arawn-engine →
      arawn-feeds dependency cycle (arawn-feeds already depends
      on arawn-engine via arawn-integrations → arawn-service).
- [x] 7 new unit tests:
      - `parse_github_scope_handles_both_schemes`
      - `repo_bind_registers_feed_record`
      - `repo_bind_is_idempotent_no_duplicate_feed`
      - `repo_bind_rejected_when_org_already_bound`
        (cross-workstream; binding not persisted; no feed)
      - `org_bind_supersedes_existing_repo_binds` (drops two
        repo feeds + bindings; emits `superseded` array)
      - `unbind_repo_scope_drops_feed`
      Workspace 1891/0 (1885 → 1891, +6).

## Status Updates

### 2026-05-18 — bind tool wired

- `GithubScope` enum + `parse_github_scope` exposed so other
  parts of the codebase can interpret a binding string.
- `upsert_repo_mirror_feed` / `delete_feed` helpers use raw
  SQL on the storage DB; no arawn-engine → arawn-feeds dep.
- Cron pickup of the new feed records relies on the
  feed-runtime's existing startup re-scan; runtime hot-add
  of a brand-new feed_id is not wired here. Worst case: a
  newly-bound repo's first poll waits until the next process
  restart, after which it's on the 30-min cadence.

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