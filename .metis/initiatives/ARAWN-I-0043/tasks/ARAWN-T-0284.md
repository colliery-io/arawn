---
id: engineevent-ceremony-variants
level: task
title: "EngineEvent::Ceremony(_) variants + broadcast wiring"
short_code: "ARAWN-T-0284"
created_at: 2026-05-15T23:45:17.370287+00:00
updated_at: 2026-05-16T00:57:17.707455+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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
## Status Updates

**2026-05-16 — implementation landed.**

**Deviation documented:** the task body and I-0043 said "Add EngineEvent::Ceremony(_) variants on the existing broadcast channel". `arawn-service::EngineEvent` is per-conversation-turn (used by `send_message`), not a server-wide broadcast — adding ceremony variants there would conflate two different surfaces. Used `arawn-service::ServerNotice` analogy instead: a typed `CeremonyEvent` broadcast channel lives inside `arawn-ceremonies`. The binary's WS layer is the bridge — it subscribes to the ceremony channel and forwards events to clients (same pattern the existing `notice_tx` uses for `ServerNotice`).

- New `crates/arawn-ceremonies/src/events.rs`:
  - `CeremonyEvent` enum (tagged JSON with `event` + `data`):
    - `TabletGenerated { tablet_id, kind, period_key }`
    - `ItemUpdated { item_id, tablet_id }` — patch_item, add_item, done-toggle all funnel through here
    - `DiaryUpdated { tablet_id }` — fired by T-0289's `upsert_diary`
    - `PatternDetected { pattern_id, iso_week, pattern_key }`
    - `PriorityConfirmed { priority_id }` — fired by I-0042's weekly plugin
  - `CeremonyEventSender` / `CeremonyEventReceiver` aliases over `tokio::sync::broadcast`. Default capacity 64.
  - `channel()` helper for tests + top-level wiring.
  - `emit(sender, event)` swallows the "no subscribers" error — broadcast send returns `Err` when no one's listening, which is an expected state until the WS bridge wires up.

- `EngineDispatcher::with_events(sender)` builder method. Dispatcher fires `TabletGenerated` after a successful commit, and `PatternDetected` for each pattern row written during detection (using the ids returned from `write_pattern_row`).

- `CeremonyService::with_events(sender)` builder method. Service fires `ItemUpdated` after `patch_item` and `add_item`.

- Senders are `Clone`-shared — the dispatcher and service hold clones of the same channel, so subscribers see events from both paths.

**Tests (6 new, 29 total):**
- 4 in `events::tests`: round-trip via channel, tagged-JSON serialisation, no-subscriber silence, multi-subscriber fan-out.
- 2 in `service::tests`: dispatch emits `TabletGenerated`; patch_item emits `ItemUpdated`.

Next: T-0285 (activity rollup pipeline).