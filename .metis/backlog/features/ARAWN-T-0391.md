---
id: wire-atlassianfeedclient-resolve
level: task
title: "Wire `AtlassianFeedClient::resolve_project()` into Jira/Confluence templates"
short_code: "ARAWN-T-0391"
created_at: 2026-05-21T14:53:35.066449+00:00
updated_at: 2026-05-21T14:53:35.066449+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
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

*To be added during implementation*
