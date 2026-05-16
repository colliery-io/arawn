---
id: calendar-attention-gather-adapters
level: task
title: "Calendar + attention gather adapters (pluggable sources)"
short_code: "ARAWN-T-0297"
created_at: 2026-05-16T14:00:00.000000+00:00
updated_at: 2026-05-16T14:00:00.000000+00:00
parent: ARAWN-I-0041
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0041
---

# Calendar + attention gather adapters

## Parent Initiative

[[ARAWN-I-0041]]

## Objective

Define two pluggable source traits in `arawn-ceremonies`
(`CalendarSource`, `AttentionSource`) and their concrete impls
backed by the existing feed / projection stores. Mirrors the
`RollupSource` pattern from [[ARAWN-T-0285]] so the daily plugin
doesn't take hard deps on `arawn-feeds` / `arawn-projections`.

## Acceptance Criteria

- [ ] New traits in `crates/arawn-ceremonies/src/gather_sources.rs`
      (or split per source):
      - `CalendarSource::events_for(date: NaiveDate) -> Result<Vec<CalEvent>, CeremonyError>`
      - `AttentionSource::since(cursor: DateTime<Utc>, cap: usize) -> Result<Vec<SignalRow>, CeremonyError>`
      Both async with `Send + Sync`.
- [ ] DTO types `CalEvent { id, title, start, end, attendees, body_excerpt }`
      and `SignalRow { id, source_kind, source_id, ts, summary, workstream }`
      with stable `id`s the daily plugin uses as citation_ids.
- [ ] Production impls:
      - `FeedsCalendarSource` over `arawn-feeds`'s calendar projection
        — wraps the projection store; ids are `event-<feed_id>-<source_id>`.
      - `ProjectionsAttentionSource` over `arawn-projections::ProjectionStore`
        — pulls gmail + slack rows newer than `cursor`, ordered DESC, capped.
- [ ] Both impls live behind the traits in the binary's wiring
      (constructed in `main.rs` once and Arc-shared with the daily
      plugin). The daily crate itself depends on neither
      arawn-feeds nor arawn-projections.
- [ ] Stub impls (`NoopCalendarSource`, `StaticAttentionSource`) for
      in-crate tests + the daily plugin's mock-LLM UAT.
- [ ] Unit tests per real impl: returns the seeded rows, respects
      cap, ids are stable.

## Implementation Notes

### Technical Approach

1. Trait crate: `arawn-ceremonies` defines the traits + DTOs. Same
   crate as `RollupSource` from [[ARAWN-T-0285]] — adds a sibling
   module.
2. Real impls live in `arawn-engine` or `arawn` (binary side) since
   they depend on arawn-feeds / arawn-projections. Decide by where
   the existing `RollupSource` impl `CentralDbWorkstreams` lives —
   match that placement.
3. Cursor for `AttentionSource::since`: the daily plugin will pass
   the prior daily tablet's `generated_at` (or "24h ago" if no prior
   tablet). Cap defaults to 10 in the daily plugin call site.

### Dependencies

- Used by [[ARAWN-T-0296]]. Can be done in parallel — T-0296 starts
  against stub impls; this task swaps in the real ones.

### Risk Considerations

- **Time-zone handling for calendar**: `events_for(date)` takes a
  `NaiveDate` but underlying calendar events have UTC + timezone
  offsets. Trait contract: "events whose local-time start falls on
  `date`." Impl resolves via the user's configured timezone (read
  from the same place the ceremony schedule timezone lives).
- **Attention payload size**: long Slack message bodies bloat the
  prompt. The trait DTO carries a `summary` field, not the raw
  body — impl is responsible for truncating to a reasonable length
  (suggest ~300 chars) before returning.

## Status Updates

*To be added during implementation*
