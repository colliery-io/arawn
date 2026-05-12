---
id: calendar-projection-calendar-events
level: task
title: "Calendar projection — calendar_events"
short_code: "ARAWN-T-0246"
created_at: 2026-05-12T03:28:19.683286+00:00
updated_at: 2026-05-12T12:52:14.599968+00:00
parent: ARAWN-I-0040
blocked_by: [ARAWN-T-0242]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Calendar projection — calendar_events

## Parent Initiative

[[ARAWN-I-0040]]

## Objective

Implement the `calendar_events` projection on top of T-0242's plumbing. Each calendar event becomes a projection row with start/end + attendee metadata, embedded + FTS-indexed.

## Scope

- `calendar_events` table: id, feed_id, source_id (event id), source_ts (event start), calendar_id, summary, description, location, start_ts, end_ts, all_day (bool), organizer, attendees (JSON), status, recurring_event_id, body_text (computed: `summary + description`), created_at/updated_at, UNIQUE(feed_id, source_id).
- FTS5 over `summary + description + location`.
- Embedding over `body_text`.
- UPSERT on `(feed_id, source_id)` — events get rescheduled / updated.
- Recurring instances: store each materialized occurrence as a separate row (preserve `recurring_event_id` for grouping).
- Mirror-to-projection adapter in `arawn-feeds::templates::calendar::*`.
- Backfill walks the existing mirror.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `calendar_events` table created with FTS + embedding, populated after `calendar-upcoming` / `calendar-archive` feed runs.
- [ ] Idempotent; UPDATE refreshes embedding when summary/description changes.
- [ ] Recurring instances stored as distinct rows with shared `recurring_event_id`.
- [ ] Backfill walks the existing mirror.
- [ ] `angreal check workspace` + `angreal check clippy` clean.

## Implementation Notes

- Time-range queries (`feed_search` later filters by date range) need an index on `start_ts`; add it explicitly.
- All-day events have a date-only start/end; normalize to ISO 8601 with `T00:00:00`.

### Dependencies

- T-0242 (projection plumbing).

## Status Updates

### 2026-05-12 — Calendar adapter landed

- `crates/arawn-projections/src/calendar.rs` — `CalendarEventProjection`. `parse_event_time` handles both `dateTime` (RFC3339) and `date` (all-day → midnight UTC) forms.
- `walk_feed_dir` reads `<feed_dir>/events/<event_id>.json` per the upcoming-archive mirror shape.
- 4 unit tests: dateTime event, all-day event normalization, multi-event walk, malformed-event skip.
- Wired into dispatcher (`calendar` provider).
- Recurring instances are stored as separate rows with `recurring_event_id` in metadata for later grouping.

`angreal check workspace` + `clippy` clean.