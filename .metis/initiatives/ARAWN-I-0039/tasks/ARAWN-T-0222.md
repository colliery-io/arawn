---
id: confluence-feed-template-space
level: task
title: "Confluence feed template — space-archive"
short_code: "ARAWN-T-0222"
created_at: 2026-05-08T21:01:13.000000+00:00
updated_at: 2026-05-08T21:01:13.000000+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

*To be added during implementation*
