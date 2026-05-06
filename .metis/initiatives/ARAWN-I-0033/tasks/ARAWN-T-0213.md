---
id: atlassian-migrate-confluence
level: task
title: "Atlassian: migrate Confluence client from deprecated v1 to v2 API"
short_code: "ARAWN-T-0213"
created_at: 2026-05-06T14:26:13.198125+00:00
updated_at: 2026-05-06T23:17:35.632937+00:00
parent: ARAWN-I-0033
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0033
---

# Atlassian: migrate Confluence client from deprecated v1 to v2 API

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0033]]

## Objective **[REQUIRED]**

Confluence tools are returning 403 with "deprecated endpoint" errors during T-0206 UAT against `sa-demo.atlassian.net`. The client in `crates/arawn-integrations/src/atlassian/client.rs:53-56` builds the Confluence base URL as `/wiki/rest/api` (v1). Atlassian has deprecated these endpoints; some sites still respond, others (incl. ours) return 403.

Migrate the entire Confluence surface to the v2 REST API at `/wiki/api/v2/...`. v2 has different request/response shapes per endpoint, so this is a touch-everything change, not a base-URL swap.

**Type:** Bug. **Priority:** P1 — Confluence half of T-0206 is unusable until this lands. Jira half is also failing UAT with HTTP 410 on `jira_search_issues` — same root cause: hand-rolled client against deprecated endpoints.

**Approach:** swap in [`jira_v3_openapi`](https://crates.io/crates/jira_v3_openapi) (auto-generated from Atlassian's OpenAPI spec, supports `oauth_access_token` config field directly) for the entire Jira surface. For Confluence, hand-update our existing reqwest-based client from `/wiki/rest/api/...` (v1) to `/wiki/api/v2/...` (v2) paths — only 5 endpoints, smaller diff than adopting `jc-conf` (30 downloads, untested in our context).

**Affected tools** (5 in `crates/arawn-integrations/src/atlassian/confluence.rs`):

| Tool | v1 endpoint (now) | v2 endpoint (target) |
|---|---|---|
| `confluence_search` | CQL via `/search` | `/pages?body-format=...&space-id=...` plus query / `/spaces` filtering |
| `confluence_get_page` | `/content/{id}?expand=body.storage` | `/pages/{id}?body-format=storage` |
| `confluence_create_page` | POST `/content` | POST `/pages` |
| `confluence_update_page` | PUT `/content/{id}` | PUT `/pages/{id}` |
| `confluence_list_spaces` | GET `/space` | GET `/spaces` |

v2 specifics that bite:
- **Cursor pagination** instead of `start`/`limit` offsets — every list endpoint changes its pagination contract.
- **Body format** is a query param `body-format=storage|atlas_doc_format|view`, not an `expand` chain.
- **Version semantics**: update_page requires the current version number in the request body to detect concurrent edits; v1 was lenient.
- **CQL is gone** — `confluence_search` needs a different strategy. The closest v2 replacement is `/pages?title=...` or `/spaces/{id}/pages?label=...`, with multi-call orchestration if the user asks for a free-text search. Worst case we keep v1 just for search until v2 grows a search endpoint.

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

## Acceptance Criteria **[REQUIRED]**

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

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

## Status Updates **[REQUIRED]**

### 2026-05-06 — Implementation complete (pending UAT)

**Jira side — generated client adoption.**

- Added `jira_v3_openapi = { version = "1.6.1", features = ["all"] }`. The crate's `models/mod.rs` re-exports each model unconditionally even though the model files are feature-gated; `features = ["all"]` works around it.
- New helper on `AtlassianClient`: `jira_config(site)` returns a `jira_v3_openapi::Configuration` with `oauth_access_token` set to a fresh access token and `base_path` set to `https://api.atlassian.com/ex/jira/{cloud_id}` (the gateway URL — generated paths append `/rest/api/3/...` themselves).
- Rewrote all 6 Jira tools to call generated functions:
  - `jira_search` → `issue_search_api::search_and_reconsile_issues_using_jql_post` (the new `/search/jql` endpoint that replaces the deprecated `/search`). Returns `next_page_token` instead of `total`.
  - `jira_get_issue` → `issues_api::get_issue` + `issues_api::get_transitions`.
  - `jira_create_issue` → `issues_api::create_issue` with `IssueUpdateDetails`.
  - `jira_update_issue` → `issues_api::edit_issue`.
  - `jira_add_comment` → `issue_comments_api::add_comment` with a `Comment` built via serde from a JSON literal carrying the ADF body.
  - `jira_transition_issue` → `issues_api::get_transitions` + `issues_api::do_transition`.
- Existing `IssueSummary` / `IssueDetail` / `CommentSummary` shapes preserved so the agent's tool-output contract doesn't change. Field extraction uses a `fields_map(&IssueBean) -> serde_json::Map` adapter so the existing JSON-pluck helpers still apply.
- Dropped the hand-rolled `jira_get` / `jira_post` / `jira_put` methods from `client.rs`. Also dropped `Product::Jira` and the now-unused `send_no_body` helper.

**Confluence side — v1 → v2 endpoints, hand-updated.**

- `client.rs` now has two Confluence base paths: v2 default (`/wiki/api/v2`) via `confluence_get/post/put`, and a `confluence_v1_get` escape hatch for endpoints with no v2 equivalent yet.
- Migrated tools:
  - `confluence_list_spaces` → `GET /spaces`. Response now includes numeric `id` alongside `key`. `SpaceSummary` gains an `id` field.
  - `confluence_get_page` → `GET /pages/{id}?body-format=storage`. v2 returns no nested space block, only `spaceId` — we resolve back to space key with one extra `/spaces` GET (best-effort, fails open).
  - `confluence_create_page` → `POST /pages` with `spaceId` (resolved from user-supplied `space_key`), `status: "current"`, body shape `{representation, value}` instead of nested `{storage: {value, representation}}`. `parentId` instead of `ancestors[]`.
  - `confluence_update_page` → `PUT /pages/{id}` with v2 body shape and required `version.number = current + 1`.
  - `confluence_search` stays on v1 (`confluence_v1_get`) — CQL has no v2 equivalent yet, and Atlassian's deprecation table still lists `/wiki/rest/api/search` as functional.

**Validation.**

- `angreal check workspace` and `angreal check clippy` clean.
- `cargo test -p arawn-integrations`: 47 passed, 0 failed.
- Full unit suite (`angreal test unit`): green.

**UAT remaining.** The same 11-tool Atlassian script. Server needs a fresh release build + restart before retrying.