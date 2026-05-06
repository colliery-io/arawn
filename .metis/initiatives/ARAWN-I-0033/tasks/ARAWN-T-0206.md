---
id: atlassian-integration-jira
level: task
title: "Atlassian integration — Jira + Confluence (OAuth, cloud_id discovery, 11 tools)"
short_code: "ARAWN-T-0206"
created_at: 2026-05-06T02:41:58.813322+00:00
updated_at: 2026-05-06T02:42:58.142515+00:00
parent: ARAWN-I-0033
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: ARAWN-I-0033
---

# Atlassian integration — Jira + Confluence (OAuth, cloud_id discovery, 11 tools)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0033]]

## Objective

Single `atlassian` integration covering both Jira and Confluence (Atlassian Cloud only — Server / Data Center is out of scope; different ecosystem). One OAuth dance, one client_id/secret, one persisted token; both tool families light up after `/connect atlassian`.

User picked option A from the design conversation — Jira + Confluence in one chunk, "now" — rather than splitting Jira (T-foundation) + Confluence follow-up. We'll land them together because they share auth + cloud_id discovery.

## Type / Priority
- Feature
- P1 — Atlassian is one of the highest-value information surfaces for personal-assistant use after Slack and Drive.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement  
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

## Acceptance Criteria

- [ ] `crates/arawn-integrations/src/atlassian/` with `integration.rs`, `client.rs`, `jira.rs`, `confluence.rs`. (Single integration; tools split into two files for clarity.)
- [ ] OAuth 2.0 (3LO) flow against `https://auth.atlassian.com`. Standard PKCE flow — works with arawn-auth's existing `OAuthClient` directly. `offline_access` scope requested so we get a refresh token.
- [ ] Post-OAuth `cloud_id` discovery: hit `https://api.atlassian.com/oauth/token/accessible-resources` after the token exchange, persist the list of `(cloud_id, name, url)` tuples in the token's `extras` field (using the same shape we built for Slack's `authed_user`). First-listed site is the default; tools take an optional `site` param to switch.
- [ ] `ARAWN_ATLASSIAN_CLIENT_ID` / `_SECRET` env vars + `[integrations.atlassian]` config block. Same precedence pattern as Slack.
- [ ] OAuth scopes (full read+write for v1):
  - `read:jira-work`, `write:jira-work`, `read:jira-user`
  - `read:confluence-content.all`, `write:confluence-content`, `read:confluence-space.summary`
  - `offline_access`
- [ ] **Jira tools (6):**
  - `jira_search({jql, max_results?, fields?})` — JQL search; returns issues with key, summary, status, assignee, priority, updated. ReadOnly.
  - `jira_get_issue({key})` — full issue including comments + available transitions. ReadOnly.
  - `jira_create_issue({project_key, summary, description, issue_type, assignee?})` — Other (mode default: ask).
  - `jira_update_issue({key, fields})` — generic update via partial-payload; FileWrite.
  - `jira_add_comment({key, body})` — Other (mode default: ask).
  - `jira_transition_issue({key, transition_name})` — resolve transition_name → transition_id via the available-transitions list, then POST. Other (mode default: ask).
- [ ] **Confluence tools (5):**
  - `confluence_search({cql, limit?})` — CQL search; returns results with id, title, space_key, type, url. ReadOnly.
  - `confluence_get_page({page_id, expand_body?})` — full page metadata + body (storage format → markdown via simple converter; agent can also request raw HTML). ReadOnly.
  - `confluence_create_page({space_key, title, body_markdown, parent_id?})` — converts markdown → Confluence storage format; Other (mode default: ask).
  - `confluence_update_page({page_id, title, body_markdown})` — fetches current version, increments, posts. Other (mode default: ask).
  - `confluence_list_spaces()` — discovery; ReadOnly.
- [ ] `Integration::capabilities_summary` reports connected sites: e.g. `"atlassian (connected; sites: acme.atlassian.net, personal.atlassian.net; 11 tools)"`.
- [ ] `docs/src/integrations/atlassian.md` — Developer Console walkthrough, scope explanation, tool reference, common JQL/CQL examples.
- [ ] Tests: parameter parsing per tool, response-shape mappers (Issue → IssueSummary, Page → PageSummary), and the cloud_id selection logic (default-to-first vs explicit-site).

## Implementation Notes

- **Service name:** `atlassian`. `/connect atlassian` runs the OAuth flow.
- **HTTP client:** roll our own with `reqwest` (already a workspace dep). The Rust Atlassian client crates are sparse and not gold-standard. We'll write thin typed-response wrappers instead of pulling in a half-baked SDK.
- **Markdown ↔ Confluence storage format:** Confluence stores pages in its own XML-flavored "storage format" (HTML-ish with custom macros). For v1: convert markdown → Confluence storage on write using a small handwritten converter (handles paragraphs, headers, lists, code blocks, links — the 80% case); convert storage → markdown on read using `pulldown-cmark`-style simple stripping. Agent can ask for raw storage format if needed.
- **JQL / CQL:** pass through to the agent. Don't try to wrap or simplify the query language — the LLM is fine constructing JQL/CQL given the tool description and a couple of examples in the docs.
- **Site selection:** persist accessible_resources alongside the token. Default site is the first returned. Tools optionally take a `site` param matching one of the persisted site URLs (e.g. `"acme.atlassian.net"`); if unset, use default.
- **Token refresh:** Atlassian tokens are short-lived (1 hour). The `offline_access` scope gives us a refresh token; arawn-auth's existing `OAuthClient::refresh` handles it. Token refresh is automatic.
- **Permission category for write ops:** matches Gmail/Drive pattern — `Other` (mode default: ask) for create/update/comment/transition operations; `FileWrite` for `jira_update_issue` (partial field update; less destructive).
- **Per-tool scope checks:** apply the same Slack/Drive Phase-1 pattern — each tool declares its required scope set, runtime check before execute returns a clean error if missing.
- **Capabilities summary:** dynamic per-turn (matches T-0204 Phase 2 pattern). Shows connected sites + lists which tools are usable based on granted scopes.

## Out of Scope (defer)

- **Atlassian Server / Data Center.** Separate auth flow, separate API base. Niche for the personal-use audience.
- **Bitbucket.** Different OAuth ecosystem under the same Atlassian brand. Separate task.
- **Jira workflow customization** (creating workflows, fields, schemes). Admin-tier surface.
- **Confluence comments / replies.** Useful but expandable later.
- **Rich-text edit modes** for Confluence beyond markdown round-trip. Acceptable v1 limitation.
- **Webhook subscriptions** for "tell me when an issue changes". Belongs to the workflow/scheduling layer once that's mature.

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates

### 2026-05-06 — Implemented (11 tools), awaiting setup + UAT

**Crate:** no Atlassian SDK in the gold-standard family — Rust Atlassian crates are sparse / unmaintained. Hand-rolled HTTP client over `reqwest` (~200 LOC) instead. Total integration is ~1,500 LOC across 4 source files plus mod.

**Layout** (`crates/arawn-integrations/src/atlassian/`):
- `integration.rs` — `AtlassianIntegration`, `AtlassianProviderConfig`, `AtlassianSite`. OAuth + post-token `accessible-resources` discovery. Sites persisted in token's `extras` field (same shape as Slack's `authed_user`).
- `client.rs` — `AtlassianClient`: refresh-aware HTTP wrapper. Substitutes `cloud_id` into the API base. Auto-refreshes 1-hour tokens via `arawn_auth::OAuthClient::refresh` + `offline_access`.
- `jira.rs` — 6 Jira tools.
- `confluence.rs` — 5 Confluence tools + a markdown↔Confluence-storage-format converter (paragraphs, headers, lists, code blocks, **bold**/*italic*/`code` — the 80% case).

**Tools (11 total):**

| Tool | Permission | Backed by |
|---|---|---|
| `jira_search` | ReadOnly | `/search?jql=...` |
| `jira_get_issue` | ReadOnly | `/issue/{key}` + `/issue/{key}/transitions` |
| `jira_create_issue` | Other (ask) | POST `/issue` (description auto-wrapped in ADF) |
| `jira_update_issue` | FileWrite | PUT `/issue/{key}` |
| `jira_add_comment` | Other (ask) | POST `/issue/{key}/comment` (ADF-wrapped) |
| `jira_transition_issue` | Other (ask) | resolves transition_name → id, POSTs |
| `confluence_search` | ReadOnly | `/search?cql=...` |
| `confluence_get_page` | ReadOnly | `/content/{id}?expand=body.storage,version,space` (storage→md) |
| `confluence_create_page` | Other (ask) | POST `/content` (md→storage) |
| `confluence_update_page` | Other (ask) | fetch current version, PUT with `version: current+1` |
| `confluence_list_spaces` | ReadOnly | GET `/space` |

**Multi-site:** OAuth response → `accessible-resources` discovery → `Vec<AtlassianSite>` persisted in token extras. Tools take optional `site` param; default is first-listed. `capabilities_summary` reports all sites.

**Per-tool scope checks** (Slack/Drive Phase-1 pattern): each tool declares required scopes, runtime check returns clean ToolError naming the gap.

**Capabilities summary** plugs into the dynamic system-prompt fragment from T-0204 phase 2: `"atlassian (connected; sites: ...; scopes: ...). Jira tools (jira_*) and Confluence tools (confluence_*) both available."`.

**Wiring:** `ARAWN_ATLASSIAN_CLIENT_ID`/`_SECRET` env or `[integrations.atlassian]` config block. Example in `~/.arawn/arawn.toml`.

**Tests:** 11 new (`atlassian::*::tests::*`). All 47 arawn-integrations tests pass. Clippy auto-fixed two minor lints in the markdown converter.

### Setup — to do before UAT

In **Atlassian Developer Console** (`developer.atlassian.com/console/myapps/`):

1. **Create** → **OAuth 2.0 (3LO) integration**. Name it `arawn`.
2. **Permissions** tab → **Add APIs**:
   - **Jira API** → add scopes: `read:jira-work`, `write:jira-work`, `read:jira-user`
   - **Confluence API** → add scopes: `read:confluence-content.all`, `write:confluence-content`, `read:confluence-space.summary`
   - **User identity API** → add scope: `offline_access`
3. **Authorization** → **OAuth 2.0 (3LO)** → Authorized redirect URL: `http://localhost:8080/oauth/callback`
4. **Settings** → copy Client ID + Secret.
5. Either env: `export ARAWN_ATLASSIAN_CLIENT_ID=... ARAWN_ATLASSIAN_CLIENT_SECRET=...`, or config:
   ```toml
   [integrations.atlassian]
   client_id = "..."
   client_secret = "..."
   ```
6. Restart `arawn serve` — should log `Atlassian integration registered (11 tools — 6 Jira, 5 Confluence)`.
7. In TUI: `/connect atlassian`. OAuth dance, browser asks which site(s) to authorize; pick yours. Server discovers `cloud_id`s via `accessible-resources`, persists, and the next agent turn sees Atlassian in `capabilities_summary`.

### UAT — to run

**Jira:**
| # | Prompt | Tool | Watch for |
|---|---|---|---|
| 1 | "search Jira for issues assigned to me that are not Done" | `jira_search` | Returns IssueSummary array; agent picks JQL like `assignee = currentUser() AND status != Done` |
| 2 | "show me the details for [a key from #1]" | `jira_get_issue` | Full issue with comments + available_transitions |
| 3 | "create a Task in [PROJECT] called 'arawn test issue' with description 'made by arawn'" | `jira_create_issue` | Permission prompt fires. Verify in Jira UI. |
| 4 | "add a comment 'this is a test from arawn' to that issue" | `jira_add_comment` | Permission prompt. Comment shows in Jira. |
| 5 | "transition that issue to In Progress" | `jira_transition_issue` | Resolves name→id. Permission prompt. |

**Confluence:**
| # | Prompt | Tool | Watch for |
|---|---|---|---|
| 6 | "list my Confluence spaces" | `confluence_list_spaces` | Returns spaces with key, name, type |
| 7 | "search Confluence for pages matching 'onboarding'" | `confluence_search` | Returns hits with id, title, space, url |
| 8 | "fetch the content of page [pick one]" | `confluence_get_page` | Body comes back as markdown (lossy from storage format) |
| 9 | "create a Confluence page in [SPACE] titled 'arawn test page' with body '# Hello\n\nThis is **bold**.'" | `confluence_create_page` | Permission prompt. Verify formatting in Confluence. |
| 10 | "update that page's body to add 'Updated by arawn'" | `confluence_update_page` | Version auto-increments. Permission prompt. |

**Multi-site test (only if you have multiple Atlassian instances):**
| 11 | "list spaces from acme.atlassian.net" | `confluence_list_spaces` with `site` arg | Switches to that site's cloud_id |

**Things to watch for:**
- Agent constructs sensible JQL/CQL from the tool descriptions — no need to teach syntax separately.
- Token refresh: leave for >1 hour, then call any tool — should silently refresh via `offline_access`. If it doesn't, refresh handling needs another look.
- Storage→markdown is lossy: pages with macros, tables, or attachments lose formatting. Acceptable for v1; agent should still get the gist.
- ADF wrapping: descriptions/comments come out as plain text only — Jira's ADF rich-content rabbit hole is deferred.

If a tool fails: paste the error. Most likely failure points are ADF formatting (Jira fields are picky), Confluence storage format edge cases, or scope misses if a permission wasn't actually saved in Developer Console.