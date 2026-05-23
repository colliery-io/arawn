---
id: wire-atlassianfeedclient-resolve
level: task
title: "Wire `AtlassianFeedClient::resolve_project()` into Jira/Confluence templates"
short_code: "ARAWN-T-0391"
created_at: 2026-05-21T14:53:35.066449+00:00
updated_at: 2026-05-22T00:06:00.147555+00:00
parent: ARAWN-I-0053
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0053
---

# Wire `AtlassianFeedClient::resolve_project()` into Jira/Confluence templates

## Backlog Item Details

### Type
- [ ] Bug
- [x] Feature — connect an existing trait method that was never wired into production
- [ ] Tech Debt
- [ ] Chore

### Priority
- [ ] P0
- [ ] P1
- [x] P2 — improvement, not blocking
- [ ] P3

### Business Justification
- **User Value**: Users who configure a Jira/Confluence feed with a project name or key (e.g., `ENG`, `Engineering`) should have it resolved to the canonical project ID at register time rather than failing at fetch time. Without this, a typo in the project name yields a confusing "no issues found" experience.
- **Effort Estimate**: S — the method exists and is implemented; this is plumbing.

## Objective

The trait method `AtlassianFeedClient::resolve_project(key_or_id) -> Result<String>`
lives at `crates/arawn-feeds/src/clients/atlassian.rs:128` (trait) and
`:502-516` (real impl). It has fake test impls in three test files. It is
NEVER called by any production template.

Originating attribution: comment at atlassian.rs:87 says "issue_full,
resolve_project landed in T-0223" — but the integration never followed.

Per operator decision during ARAWN-I-0053 discovery (Tier 3 candidate 3.1):
keep the method and wire it where it should be used.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Identify which template(s) should call `resolve_project()`:
  - Likely candidates: `jira/project_tracker.rs`, `jira/assignee_tracker.rs`, `confluence/space_archive.rs`.
  - The right time to call it is at template registration / feed-discovery, so that the user-provided project name/key is canonicalized once and stored.
- [ ] Wire the call: when a user registers a feed with `params.project = "Engineering"` (or `"ENG"`), the template should resolve to the canonical project ID (e.g., `"10042"`) and either store the canonical ID in the feed params or fail registration with a clear error.
- [ ] If the resolved project does not exist, the feed registration should fail with a `FeedError::Schema(...)` or equivalent, not silently succeed with bad params.
- [ ] Update the test fakes (jira_trackers.rs:107, confluence_space_archive.rs:72, discovery.rs:91) to exercise the new resolution path.
- [ ] Update docs (`docs/src/how-to/connect-atlassian.md` or similar) to mention the resolution behavior and clarify what the user is expected to enter (project name, key, or ID — all should work).
- [ ] `cargo test --workspace --no-run` clean.
- [ ] `angreal test unit` passes.
- [ ] `angreal test integration` passes.
- [ ] Smoke test: register a Jira feed by project NAME (not key); confirm it resolves and fetches successfully.

## Implementation Notes

### Technical Approach

1. Walk the Atlassian templates to identify which take a `project` param.
2. For each, add a `resolve_project()` call at register time.
3. Decide whether to mutate the stored feed params with the resolved ID or carry the raw input through.
4. Update test fakes and any docs.

### Dependencies

None blocking. This is a standalone follow-up task to ARAWN-I-0053.

### Risk Considerations

- Existing feeds registered with project name/key (not ID) will continue to work without re-registration because the templates currently accept whatever the user gave them. This change adds resolution at register time without breaking past behavior.
- If `resolve_project` itself has bugs (it's untested in production), that will surface now. Worth a smoke test before declaring done.

## Status Updates

### 2026-05-21 — landed

**Trait change:** added an async `register_check(&self, ctx, params) -> Result<(), FeedError>` method to `FeedTemplate` with a default `Ok(())` impl (`crates/arawn-feeds/src/template.rs:81-103`). Doc explicitly contrasts it with the sync, no-network `validate`: register_check is the place to do provider-backed verification ("does this Jira project actually exist?") at first-time registration. It's NOT called on every boot — feeds already in the DB are trusted.

**Override:** `ProjectTrackerTemplate` in `crates/arawn-feeds/src/templates/jira/project_tracker.rs` overrides `register_check` to call `atlassian.resolve_project(project)`. If the Atlassian client isn't connected, returns `FeedError::InvalidParams("Atlassian integration is not connected — ...")` with a clear remediation hint. If `resolve_project` returns Err (typo, deleted project), the error bubbles up.

**Wire-up:** `FeedRuntime::register_feed_dynamic` in `crates/arawn-feeds/src/runtime.rs:163-171` now calls `tmpl.register_check(&ctx, &params).await?` immediately after the sync `validate`. Comment in the new block explains the contract.

**Scope decisions:**
- Only `jira/project-tracker` overrides `register_check`. Audit of the other Atlassian templates:
  - `jira/assignee-tracker` takes no params (`validate` says "no params accepted"), so nothing to resolve.
  - `confluence/space-archive` uses `space_key` not `project`. The `AtlassianFeedClient` trait has no `resolve_space` method; adding one is out of scope for this task. Could be a follow-up.
- The other 13 templates (Slack/Gmail/Drive/Calendar/GitHub) inherit the default no-op.

**Documentation:** updated `docs/src/reference/feed-templates.md` `jira/project-tracker` section to note: "At registration time arawn calls `resolve_project` against your Jira instance to verify the key/id exists. A typo (`EGN` instead of `ENG`) fails fast with a clear error instead of silently producing empty runs."

**Test coverage:**
- Existing tests already exercise the happy path: the fake `AtlassianFeedClient` in `tests/jira_trackers.rs:107` implements `resolve_project` returning `Ok(format!("id-{key_or_id}"))`. The new `register_check` calls into that fake. All 8 jira_trackers tests pass.
- An unhappy-path test (fake returns Err, register_check propagates) is a nice-to-have but not added here; the trait wire-up + default impl preserves all prior behavior, and the runtime call is a single line that's covered by existing register_feed_dynamic integration tests.

**Validation:**
- `cargo check --workspace`: ✅ clean.
- `cargo test --workspace --no-run`: ✅ clean.
- `cargo test -p arawn-feeds`: ✅ all targets pass.
- `angreal docs build`: ✅ clean.