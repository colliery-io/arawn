---
id: confluence-space-archive-feed-401
level: task
title: "Confluence space-archive feed: 401 scope mismatch on CQL search + page body"
short_code: "ARAWN-T-0229"
created_at: 2026-05-09T00:00:00+00:00
updated_at: 2026-05-10T14:43:28.022527+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

### 2026-05-09 — first hypothesis disproven; real cause still unknown

Initial hypothesis was that the persisted token was minted by an older arawn build with fewer scopes. Added a `missing_scopes()` method on `AtlassianIntegration` that compares `granted_scopes()` against the current `ATLASSIAN_OAUTH_SCOPES` list, plus a startup `warn!` that fires if any are missing.

**Result:** the warning does NOT fire for the affected user's token. Their token has every scope the current binary requests, including the v2 grants (`read:page:confluence`, `read:space:confluence`).

So the 401 `scope does not match` is coming from an endpoint that needs a scope NOT in our requested list. Possibilities to investigate:

1. **CQL `/search` may need `read:confluence-content.summary`** specifically — different scope than `read:confluence-content.all`. The `.all` is documented as broader, but Atlassian may treat them as orthogonal rather than inclusive.
2. **CQL `/search` may need `search:confluence`** — a search-specific granular scope added when Atlassian split the legacy scopes.
3. **`/wiki/api/v2/pages/{id}?body-format=storage` may need an additional scope when the body format is `storage`**, beyond `read:page:confluence`.
4. **Token-context mismatch.** The `confluence_v1_get` helper might pick the bot-app context where the feed needs the user-3LO context (or vice versa). Worth checking: does the existing `confluence_search` tool (which calls the same `/search` endpoint) succeed for this user? If yes, the difference is the auth context, not the scope.

### Investigation plan

- [ ] User to test: does `/confluence_search cql=type=page AND space=SD` work via the tool path? If yes, the diagnosis is auth-context, not scope.
- [ ] Inspect the persisted token's `scope` field directly to confirm which scopes the user actually has.
- [ ] Try the failing endpoint(s) by hand with the persisted token (`curl -H 'Authorization: Bearer ...'`) to isolate which call returns 401 — is it `/search` or `/pages/{id}` or both? The error in the log is from one of them; we don't know which.
- [ ] Check Atlassian developer console for the OAuth app's actual granted scopes vs what we request — sometimes Atlassian downgrades scope grants silently.

### Startup-warning safeguard landed regardless

`AtlassianIntegration::missing_scopes()` and the `[atlassian]` startup warn message landed in this commit. They'll catch the *original* hypothesis (older token + fewer scopes) for any user who has one. Doesn't help the current case but prevents the same triage path next time.

### Workaround for the user (current)

`/disconnect atlassian` + `/connect atlassian` doesn't fix this case (token already has the right scopes), so the `sd` feed will keep failing until the actual scope is identified. `/feeds rm sd` to clear the failing schedule, or `/feeds pause sd` to stop the cron firings until the fix lands.

### 2026-05-10 — fixed: scope additions + fresh /connect

The three guessed-at scopes did the right thing. After landing them in `ATLASSIAN_OAUTH_SCOPES` and the user re-running `/disconnect atlassian` + `/connect atlassian` (twice — first attempt's accessible-resources discovery silently failed and stored an empty sites list), all three atlassian feeds came up clean:

- `confluence/space-archive sd` — `status=ok`, 5 page dirs, 56K, cursor advancing.
- `jira/assignee-tracker me` — `status=no-new-items`, 46 issue dirs, 704K.
- `jira/project-tracker API` — `status=no-new-items`, 8 issue dirs, 148K.

The missing scopes were:
- `search:confluence` — granular CQL `/search` scope.
- `read:confluence-content.summary` — required alongside `.all` for some v1 paths.
- `read:content-details:confluence` — needed by v2 `?body-format=storage`.

The `missing_scopes()` startup-warning safeguard from the first commit also stays in place to catch the older-token-with-fewer-scopes case for future users.

Side note worth filing as its own follow-up: the silent-fail path in `AtlassianIntegration::connect` when accessible-resources discovery fails. The first reconnect persisted an empty sites list, the next `feed_run` failed with "no accessible sites — reconnect", and the user had to reconnect again. The integration should either retry discovery or refuse to persist the token if discovery fails. Filed separately.