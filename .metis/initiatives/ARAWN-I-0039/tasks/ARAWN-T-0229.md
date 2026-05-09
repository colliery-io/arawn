---
id: confluence-space-archive-feed-401
level: task
title: "Confluence space-archive feed: 401 scope mismatch on CQL search + page body"
short_code: "ARAWN-T-0229"
created_at: 2026-05-09T00:00:00+00:00
updated_at: 2026-05-09T00:00:00+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0039
---

# Confluence space-archive feed: 401 scope mismatch on CQL search + page body

## Parent Initiative

[[ARAWN-I-0039]]

## Severity

P1 — `confluence/space-archive` is unusable today. Found during T-0218 UAT.

## Reproduction

1. `/connect atlassian` (grants the standard Atlassian OAuth scope set used by the Jira+Confluence tools).
2. `/watch confluence/space-archive sd space_key=SD`.
3. Wait for next */30 cron firing.
4. Observe in server log:

```
ERROR feed run failed feed_id=sd
  error=auth failed: HTTP 401 Unauthorized:
  {"code":401,"message":"Unauthorized; scope does not match"}
```

## Diagnosis

The `confluence/space-archive` template (in `templates/confluence/space_archive.rs`) calls two Confluence endpoints via the `AtlassianFeedClient`:

1. `space_pages_modified_since` — wraps `confluence_v1_get("/search", ...)` (CQL search). Per Atlassian's deprecation table, CQL has no v2 equivalent and v1 `/search` remains supported. Requires `read:confluence-content.summary` (or the equivalent v2 search scope when one ships).

2. `page_body_storage` — wraps `confluence_get("/pages/{id}?body-format=storage", ...)`. Requires the v2 page-read scope: `read:page:confluence`.

The existing Atlassian OAuth scope grant (in `arawn-integrations/src/atlassian/integration.rs::oauth_config`) was written for the **tool** surface (`confluence_search`, `confluence_get_page`, etc.) and either:

- includes scopes that work for the tools but happen to differ from what feeds need, or
- omits one of the v2 read scopes the page body fetch needs.

The `scope does not match` error is Atlassian's specific phrasing for "this token has scopes A, but you asked for an endpoint that needs scope B" — it's not a token-expiry or grant-revoked failure; it's a mismatch.

## Action

- [ ] Audit `oauth_config` in `arawn-integrations/src/atlassian/integration.rs`. List the current scope set verbatim in this task's status updates.
- [ ] Cross-reference each tool + each feed-trait method against [Atlassian's scope reference](https://developer.atlassian.com/cloud/confluence/scopes-for-oauth-2-3LO-and-forge-apps/). Build a complete required-scope set covering:
  - Existing Confluence tools: search, get_page, create_page, update_page, list_spaces.
  - Feed methods: `space_pages_modified_since` (CQL `/search`), `page_body_storage` (v2 `/pages/{id}`), `list_confluence_spaces` (v2 `/spaces`).
  - Existing Jira tools (search, get_issue, create, edit, transition, add_comment).
  - Feed methods on the Jira side: `jql_search`, `issue_full`, `resolve_project`, `list_jira_projects`.
- [ ] Update the requested-scope list in `oauth_config`. Likely additions: `read:page:confluence`, `read:space:confluence`, plus whichever CQL search scope is missing.
- [ ] User has to **re-run `/connect atlassian`** after the change to get a token with the new scope set. Document this in the task status — re-grant is unavoidable when scopes expand.
- [ ] Verify the fix by registering a fresh `confluence/space-archive` feed and confirming it produces non-empty `page.json` + `body.storage.xml` files for each page in the space.
- [ ] If the v1 CQL `/search` endpoint is being deprecated entirely, file a separate task to migrate `space_pages_modified_since` to whatever v2 surface lands. (Not blocking this task — v1 still works once the right scope is granted.)

## Workaround

None. `confluence/space-archive` is unusable until this is fixed. `/feeds rm sd` to clear the failing feed; `/watch list confluence/space-archive` (the picker) still works because that path uses `list_confluence_spaces` which may or may not have the same gap — verify during the audit.

## Related

- T-0222 introduced this template + the `AtlassianFeedClient` trait surface.
- T-0224 added the discovery picker (`list_confluence_spaces`) which may have the same scope hole.

## Status Updates

*To be added during implementation*
