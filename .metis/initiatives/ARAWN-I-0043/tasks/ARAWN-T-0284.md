---
---
id: engineevent-ceremony-variants
level: task
title: "EngineEvent::Ceremony(_) variants + broadcast wiring"
short_code: "ARAWN-T-0284"
created_at: 2026-05-15T23:45:17.370287+00:00
updated_at: 2026-05-15T23:45:17.370287+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0043
---

# EngineEvent::Ceremony broadcast variants

## Goal
Add a new `EngineEvent::Ceremony(CeremonyEvent)` variant to the existing broadcast channel so subscribers (TUI, future GUI clients) refresh on state changes.

## Reference
I-0043 Design Decisions §2.

## Acceptance
- New `CeremonyEvent` enum: `TabletGenerated { tablet_id, kind, period_key }`, `ItemUpdated { item_id, tablet_id }`, `DiaryUpdated { tablet_id }`, `PatternDetected { pattern_id, iso_week }`, `PriorityConfirmed { priority_id }`.
- `EngineEvent::Ceremony(CeremonyEvent)` lands on the existing broadcast channel.
- Engine publishes on tablet completion (T-0282) + on every successful `patch_item` / `add_item` (T-0283) + on `upsert_diary` (T-0289).
- TUI snippet test that subscribes, drives an RPC `add_item`, and asserts the `ItemUpdated` event arrives.

## Out of scope
TUI re-render plumbing — the `/retro` client (T-0290) handles its own subscription.
