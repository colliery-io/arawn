---
id: redirect-link-shortener-encode
level: task
title: "Redirect-link shortener — encode token-heavy URLs to placeholders"
short_code: "ARAWN-T-0270"
created_at: 2026-05-15T14:12:00.508899+00:00
updated_at: 2026-05-15T14:12:00.508899+00:00
parent: ARAWN-I-0044
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0044
---

# Redirect-link shortener

## Tier
Tier 2-late — deferred. Token usage is not measurable pain today; pick this up only when token usage tracker (`T-0277`) shows a specific inbound boundary blowing context with tracking URLs, or when a concrete inbound boundary starts blowing context with tracking URLs. Still pairs with TokenJuice (`T-0274`) when both eventually land.

## Reference
`/tmp/openhuman/src/openhuman/redirect_links/`. Long tracking URLs like `trip.com/forward/...?bizData=...` get encoded to `openhuman://link/<id>` on inbound; full URL kept in SQLite; expanded back on outbound.

## Goal
On inbound text (channels, feeds, web-fetch results) URLs longer than a threshold get replaced with `arawn://link/<short-id>` placeholders. Full URLs persist in a local SQLite table (in the existing data dir). On outbound (responses sent back to user / via channels) the placeholders expand back.

## Acceptance
- New `crates/arawn-engine/src/redirect_links.rs` (or its own crate if cleaner) with `encode_inbound(text) -> text` and `decode_outbound(text) -> text` + store with id, url, created_at, last_used_at.
- Threshold configurable; default 200 chars OR contains `?` with >5 query params.
- Wired into feed ingestion and web-fetch tool output as the inbound boundary.
- Wired into the TUI / channel send path as the outbound boundary.
- Tests cover round-trip, idempotency, and the "placeholder never leaks to user" invariant.

## Out of scope
Cross-session persistence semantics beyond a simple `last_used_at` GC pass.
