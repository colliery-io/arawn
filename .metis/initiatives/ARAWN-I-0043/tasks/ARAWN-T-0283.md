---
---
id: ceremonies-ws-rpc-method-registry
level: task
title: "ceremonies.* WS-RPC method registry — shared read/mutate surface"
short_code: "ARAWN-T-0283"
created_at: 2026-05-15T23:45:12.627229+00:00
updated_at: 2026-05-15T23:45:12.627229+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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
