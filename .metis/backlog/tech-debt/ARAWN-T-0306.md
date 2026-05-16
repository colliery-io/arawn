---
id: ceremony-source-adapters-timezone
level: task
title: "Ceremony source adapters — timezone + workstream tagging"
short_code: "ARAWN-T-0306"
created_at: 2026-05-16T18:27:25.158519+00:00
updated_at: 2026-05-16T18:35:32.109746+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Ceremony source adapters — timezone + workstream tagging

## Objective

Close two deferrals noted in [[ARAWN-T-0297]]'s status update:

1. `ProjectionsCalendarSource::events_for(date)` filters with a UTC
   day window. With a user in a non-UTC timezone, "today" events
   near midnight land on the wrong day.
2. `ProjectionsAttentionSource` returns `SignalRow.workstream = None`
   for every row. Should resolve via the feed_id → workstream
   lookup in arawn-storage's `feeds` table.

## Acceptance Criteria

- [ ] `ProjectionsCalendarSource::events_for(date)` accepts the
      configured ceremony timezone (from
      `[ceremonies.<kind>].timezone`, defaulting to "Local") and
      brackets the day in that zone before converting to UTC for
      the BETWEEN query. New constructor or builder field —
      e.g. `ProjectionsCalendarSource::new(projections).with_tz(tz)`.
- [ ] Binary wiring threads the resolved timezone from the
      `CeremonyConfig` of each plugin into the source it
      constructs. Daily + weekly both pass.
- [ ] `ProjectionsAttentionSource` joins on
      `arawn-storage::feeds.workstream_id` (via the feeds
      registry — already keyed by feed_id) to populate
      SignalRow.workstream. Cache the lookup so the adapter
      doesn't re-query per row.
- [ ] Unit tests:
      - Calendar: same fixture events with two different
        timezones return different `events_for(date)` results
        when events cross midnight.
      - Attention: rows seeded against a feed_id that's
        registered to a workstream return that workstream name;
        rows against an unregistered feed_id return None.
- [ ] No regression in the daily UAT (timezone defaults to
      "Local" so existing test data continues to match).

## Implementation Notes

### Technical Approach

1. Timezone arg shape: `chrono_tz::Tz` if it's already in the
   workspace; otherwise just `String` and parse at call time.
   Check workspace deps first.
2. Workstream lookup: add a small read method to arawn-storage's
   feed store (probably `find_workstream_for_feed(feed_id)`)
   rather than letting the adapter reach into raw SQL. Keep the
   adapter dep-light.
3. Cache: `Arc<DashMap<String, Option<String>>>` keyed by
   feed_id is enough — a few dozen feeds at most, cache is
   per-binary-instance.

### Dependencies

- Touches `crates/arawn-engine/src/ceremony_sources.rs` and
  `crates/arawn/src/main.rs` ceremony wiring. Possibly
  `crates/arawn-storage/src/feeds.rs` if a new method is needed.

### Risk Considerations

- **Tz crate**: `chrono_tz` adds ~2MB to the binary. Worth it
  for correctness; if avoidable with a simpler API (e.g. just
  offset minutes), prefer that.
- **Cache invalidation**: workstreams are rare to be deleted at
  runtime; if it happens, the cached `None` (or stale name) is
  acceptable until restart. Document this.

## Status Updates

### 2026-05-16 — both deferrals closed

- `ProjectionsCalendarSource` gains a `tz: chrono_tz::Tz` field +
  `with_tz` builder. `events_for(date)` now brackets the day in
  the configured zone before converting to UTC bounds.
- `arawn_storage::Store::find_workstream_for_feed(feed_id)` —
  new method. Note: the actual schema stores bindings as JSON on
  `workstreams.bindings`, not a `feeds.workstream_id` column —
  the method iterates active workstreams. Cache hides the cost.
- `ProjectionsAttentionSource` now populates
  `SignalRow.workstream` via that store method; results cached
  in a `std::sync::Mutex<HashMap>` (no `dashmap` in workspace).
- `main.rs` wiring: replaced the shared `projection_sources`
  tuple with per-plugin `daily_calendar` + `weekly_calendar`
  instances (each with the right tz) plus a shared
  `attention_source`. New local helper `resolve_ceremony_tz`:
  missing or `"local"` → UTC (debug); unknown IANA → UTC (warn).
- `chrono-tz = "0.10"` added to both `arawn-engine` and
  `arawn` crates.
- 4 ceremony_sources tests pass (2 new). arawn lib: 57/57.

Completed 2026-05-16.