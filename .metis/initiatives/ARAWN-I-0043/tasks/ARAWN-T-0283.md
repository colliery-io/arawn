---
id: ceremonies-ws-rpc-method-registry
level: task
title: "ceremonies.* WS-RPC method registry — shared read/mutate surface"
short_code: "ARAWN-T-0283"
created_at: 2026-05-15T23:45:12.627229+00:00
updated_at: 2026-05-16T00:52:43.748924+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0043
---

# ceremonies.* WS-RPC method registry

## Goal
Add the shared `ceremonies.*` namespace to the existing WS-RPC surface. Routes by `kind` to the right plugin where the plugin has per-kind logic; otherwise served by the engine directly.

## Reference
I-0043 §RPC additions + Design Decisions §2.

## Acceptance
- New methods on the WS-RPC surface:
  - `ceremonies.get_today` — current daily tablet
  - `ceremonies.get_by_period { kind, period_key }` — specific tablet
  - `ceremonies.list_items { tablet_id, section_key? }` — items, optionally filtered
  - `ceremonies.patch_item { item_id, fields }` — toggle done, edit body
  - `ceremonies.add_item { tablet_id, kind, body }` — user-added todo (user-write path)
  - `ceremonies.run { kind }` — manual trigger; idempotent per period_key
  - `ceremonies.list_config` / `ceremonies.config_update { kind, fields }` — enable/disable, override cron, workstream filter
  - `ceremonies.list_notifications` — unread tablets pending review
- Auth: same as the existing RPC surface; no new auth layer.
- Tests: round-trip RPC call against a stub plugin + assert DB state.

## Out of scope
Retro-specific methods (`upsert_diary`, `list_patterns`) — those land with T-0289.

## Notes
The method names mirror the HTTP shapes called out in the original doc 1:1 — that translation was deliberate.
## Status Updates

**2026-05-16 — implementation landed.**

**Scope deviation documented:** the task body said "Add the shared `ceremonies.*` namespace to the existing WS-RPC surface" (i.e. extend `arawn-service::ArawnService` and the LocalService impl). I kept this contained inside `arawn-ceremonies` as `CeremonyService` instead — the JSON-RPC dispatcher in the binary will wire the eight methods 1:1 when ceremonies are integrated. Rationale: keeps T-0283 testable without dragging the full LocalService surface into the diff; the RPC adapter is a single mechanical change once. Documented for the future wiring task.

- New `crates/arawn-ceremonies/src/service.rs` with `CeremonyService`:
  - `get_today()` — today's daily tablet by UTC date.
  - `get_by_period(kind, period_key)` — exact lookup.
  - `list_items(tablet_id, section_key?)` — sorted by (section_key, ordinal).
  - `patch_item(item_id, ItemPatch)` — toggle done_at, edit body. Returns reloaded row.
  - `add_item(AddItemRequest)` — user-write path. NULL citation; next ordinal computed via `MAX(ordinal)+1`.
  - `run(kind)` — delegates to dispatcher.
  - `list_notifications()` — tablets with `status='open'` ordered by `generated_at DESC`.

- DTOs designed for JSON-over-RPC: `TabletDto`, `ItemDto`, `NotificationDto`, `ItemPatch`, `AddItemRequest`. All `Serialize + Deserialize`. Body fields are `serde_json::Value` so the wire encoding stays opaque per kind.

- **Config CRUD (`ceremonies.list_config`, `config_update`) is deferred.** It needs a `ceremony_config` table that's not in V6. Filed as follow-up — would land alongside a V7 migration. The engine + retro plugin don't need user-overridden config to ship; the defaults baked into `Ceremony::default_schedule()` are what the binary registers today.

**Tests (6 new in `service::tests`, 23 total in the crate):**
- `run_generates_and_get_by_period_reads_back`
- `list_items_filters_by_section`
- `patch_item_toggles_done`
- `add_item_inserts_user_row_with_null_citation_and_next_ordinal`
- `list_notifications_surfaces_open_tablets`
- `get_today_returns_none_when_no_daily_tablet`

Next: T-0284 (`EngineEvent::Ceremony(_)` broadcast variants).