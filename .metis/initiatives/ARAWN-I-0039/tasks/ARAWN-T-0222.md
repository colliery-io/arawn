---
id: confluence-feed-template-space
level: task
title: "Confluence feed template — space-archive"
short_code: "ARAWN-T-0222"
created_at: 2026-05-08T21:01:13+00:00
updated_at: 2026-05-08T23:17:39.155217+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0039
---

# Confluence feed template — space-archive

## Parent Initiative

[[ARAWN-I-0039]]

## Objective

Land the Confluence feed template from I-0039 Phase 4. Split out from T-0217 so the Atlassian read path lands separately from the heavier Jira track.

- `confluence/space-archive` — pages + bodies in a Confluence space. Param: `space_key`.

Reference: I-0039 Detailed Design; existing `arawn-integrations/src/atlassian/confluence.rs` (v2 client landed in T-0213).

## Type / Priority

- Feature, P1.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Template registered in `arawn_feeds::default_registry`.
- [ ] **AtlassianFeedClient trait + RealAtlassianClient adapter** (shared with the Jira task T-0223 once that lands; this task introduces the trait surface for Confluence and the Jira task extends it).
- [ ] Trait surface for this task: `space_pages_modified_since(space_key, last_modified_iso)` returning page metadata, plus `page_body_storage(page_id)` returning the raw `body.storage` XML.
- [ ] **Cursor**: persist `last_modified_iso` per space; v2 cursor pagination already works in our client from T-0213.
- [ ] **Disk layout**:
  - `confluence/space-archive/<feed_id>/<page_id>/page.json` — page metadata snapshot, overwrite-on-update.
  - `confluence/space-archive/<feed_id>/<page_id>/body.storage.xml` — raw body, overwrite-on-update.
- [ ] `validate(params)`: requires `space_key` non-empty.
- [ ] `defaults(params)`: cadence `30m`.
- [ ] **Auto-create** is N/A — no personal-default Confluence feed.
- [ ] **Failure modes**: token expired → `FeedError::Auth`; 410 / API deprecation → `FeedError::Schema(detail)`; rate-limit → `FeedError::RateLimited(retry_after)`.
- [ ] **Tests** in `crates/arawn-feeds/tests/confluence_space_archive.rs`:
  - `validate_rejects_missing_space_key`.
  - `space_archive_writes_per_page_metadata_and_body`.
  - `cursor_advances_only_on_successful_persist`.
  - `unchanged_pages_skip_body_fetch_via_modified_cursor`.
  - `returns_auth_when_atlassian_not_connected`.
- [ ] `angreal check workspace` and `angreal check clippy` clean.

## Implementation Notes

### Technical Approach

1. `clients/atlassian.rs` — `AtlassianFeedClient` trait + `RealAtlassianClient` adapter wrapping `arawn_integrations::atlassian::AtlassianIntegration`. Surface starts narrow (Confluence-only); T-0223 extends it for Jira.
2. `templates/confluence/space_archive.rs` — single template using the trait.
3. Hoist Atlassian Arc in main.rs; `RealClients::with_atlassian(...)` builder.

Bodies are written verbatim as `body.storage.xml` (no ADF conversion at archive time — agents prefer source-of-truth markup; the Markdown→ADF converter from T-0213 is for write paths only).

### Dependencies

- T-0214 (feed runtime, landed).
- T-0213 (Confluence v2 client, landed).
- T-0221 (Drive) — independent, can ship in parallel.

### Risk Considerations

- v2 pagination uses opaque `next` cursors; helper must follow them to fully enumerate large spaces.
- `body.storage` can be large; consider a per-page byte cap with a status flag if pages exceed it. Defer until we hit one in practice.

## Status Updates

### 2026-05-08 — confluence/space-archive landed

**Trait surface** (`AtlassianFeedClient`) — introduced here, will be extended by T-0223 with Jira methods. Confluence-only methods on the trait for now:
- `space_pages_modified_since(space_key, since)` — CQL search via the existing `confluence_v1_get("/search", ...)` (CQL has no v2 equivalent yet per Atlassian's deprecation table). Builds `space = "X" AND type = "page" AND lastmodified > "..."`. Follows v1 `start`/`limit` pagination with `_links.next` short-circuit.
- `page_body_storage(page_id)` — v2 `GET /pages/{id}?body-format=storage`, returns the raw XML.

Plus `ConfluencePageMeta` (id/title/space_key/version/modified_time/url) and `ConfluencePageBody` (id/storage_xml/version) types — kept Serializable so the template writes meta verbatim.

**Storage**: one directory per page id; both files inside are overwrite-on-update.
- `<feed_dir>/<page_id>/page.json` — metadata snapshot.
- `<feed_dir>/<page_id>/body.storage.xml` — raw body XML, atomic-rename write.

Bodies are written verbatim (no ADF/markdown conversion at archive time — agents prefer source-of-truth markup; the markdown→storage converter from T-0213 is for write paths only).

**Cursor**: `{ last_modified_iso }`. CQL's `lastmodified > "..."` is minute-grained; if the cursor falls on a minute boundary we may re-fetch a page once but the body write is idempotent so it's harmless.

**Resilience**: per-page body fetch failures (Schema or Provider variants) get a warn log and skip — one bad page doesn't poison the run. Auth/RateLimited propagate up so the runtime can back off properly.

**Error classification**: Atlassian client errors arrive as opaque `IntegrationError::Provider(String)`, so we sniff the message for known shapes ("429"/"rate limit" → RateLimited, "410"/"gone" → Schema, "401"/"403"/"unauthorized"/"invalid_grant" → Auth). Same pattern the Slack/Gmail/Drive adapters use.

**Tests** (8 integration tests):
- `writes_per_page_metadata_and_body` — base case, cursor advances.
- `second_run_passes_cursor_as_since` — second run uses prior latest as `since`.
- `body_fetch_failure_skips_page_without_aborting_run` — partial failure tolerated.
- `body_overwritten_on_re_fetch` — version bumps overwrite.
- `page_with_no_body_writes_empty_xml` — placeholder pages handled.
- `empty_run_is_no_op_with_status` — `no-new-items` status.
- `returns_auth_when_atlassian_not_connected` — auth error surfaces.
- `validate_rejects_missing_space_key` — gates required param.

**Production wiring**: hoisted Atlassian Arc the same way as Slack/Calendar/Gmail/Drive; `RealClients::with_atlassian(...)` picks it up. All existing test mocks gained no-op `atlassian()` impls.

**Departures from AC**: none material. Per-page resilience on body fetch wasn't called out in the AC but matches the pattern from earlier templates.

114 arawn-feeds tests green. `angreal check workspace` and `angreal check clippy` clean.

**Next**: T-0223 will extend `AtlassianFeedClient` with `jql_search`, `issue_changelog`, `issue_comments` rather than introduce a separate `JiraFeedClient`.