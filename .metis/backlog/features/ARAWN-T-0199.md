---
id: surface-hot-reload-status-plugins
level: task
title: "Surface hot-reload status (plugins, config) in TUI"
short_code: "ARAWN-T-0199"
created_at: 2026-05-02T00:00:00+00:00
updated_at: 2026-05-02T18:11:06.611989+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Surface hot-reload status (plugins, config) in TUI

## Objective

The plugin runtime watches `plugins/cache/` and reloads on change; the config watcher reloads `arawn.toml` on change. Both fire silently — log lines go to the server log only. A user editing a plugin can't tell whether their change took effect. A failed reload is invisible.

## Type / Priority
- Feature
- P3 — Polish. Not blocking, but the silence makes the system feel mysterious.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Plugin reload events emit a TUI-visible status: brief banner / status-line message of the form "plugins reloaded (N skills, M agents)" on success, or "plugin reload FAILED: <error>" on failure.
- [ ] Config reload events similarly visible.
- [ ] No firehose: rapid-fire reloads (e.g. an editor saving twice) coalesce into a single notification.
- [ ] Failures persist until acknowledged / dismissed (don't get scrolled away by streaming text); successes auto-fade after a few seconds.

## Implementation Notes

- Plugin watcher: `crates/arawn-engine/src/plugins/runtime.rs` (per code-index). Config watcher: `crates/arawn/src/config_watcher.rs`.
- Surface as a new `EngineEvent` variant (e.g. `SystemNotice { level, message }`) so the existing WS forwarding handles it for free.
- TUI status-line area is the natural home; if there isn't one yet, a transient banner above the input is fine.
- Don't build a full notification center — single-line banner is enough.

## Status Updates

### 2026-05-02 — Implemented

**Architecture decision:** rejected the ticket's suggested "new EngineEvent variant" — engine events are per-conversation-turn and are returned through `send_message`. Hot-reload notices are server-wide and asynchronous. Built a dedicated `ServerNotice` envelope + broadcast channel instead.

**Changes:**
- `arawn-service`: new `ServerNotice { level, category, message, timestamp }` type.
- `LocalService` owns a `tokio::sync::broadcast::Sender<ServerNotice>` (cap 64; overflow drops oldest, the right behavior for fire-and-forget notifications). Exposes `subscribe_notices()` and `notice_sender()`.
- `PluginRuntime::watch` and `ConfigWatcher` both gain optional `Arc<dyn Fn(bool, String) + Send + Sync>` notify callbacks. Wired in `main.rs` to push `ServerNotice`s with the right `category` ("plugin_reload" / "config_reload") and `level` ("info" / "error").
- `ws_server::handle_connection` was restructured to `tokio::select!` between socket reads and the broadcast subscription, forwarding notices as `{"event": "SystemNotice", "data": {...}}` JSON. `RecvError::Lagged` is logged and skipped.
- TUI: new `parse_system_notice` recognizes the envelope; both message-handling sites in `event_loop` route notices to `apply_system_notice`, which pushes a system-role chat entry of the form `ℹ [plugin_reload] plugins reloaded: 2 plugin(s), 5 skill(s), 1 agent(s)` (or `✗ [config_reload] reload FAILED: ...`).

**Plugin watcher reorder:** `plugin_runtime.watch(...)` was previously spawned before `service` existed. Moved the spawn to right after service construction so we can wire it into `service.notice_sender()`.

**Tests** (`crates/arawn-tui/src/ws_client.rs::tests`, all green):
- `parses_well_formed_system_notice`
- `rejects_engine_event_envelope`
- `rejects_response_envelope`
- `rejects_malformed_json`

893 workspace lib tests pass; 0 clippy warnings.

**Acceptance criteria status:**
- [x] Plugin reload events emit a TUI-visible status (success + failure).
- [x] Config reload events emit a TUI-visible status. Today's `reload()` doesn't have user-facing failure paths (load is best-effort and falls back); the wiring is in place via `is_error: true` for when one's added.
- [x] No firehose: watchers already debounce at the source (1s plugin, 500ms config); broadcast layer caps at 64 with drop-oldest.
- [ ] **Deferred:** "failures persist until acknowledged / successes auto-fade" — current implementation puts both into chat history (visible until scrolled). Real fade-out requires a separate notification UI surface that doesn't exist yet. Polish follow-up; `apply_system_notice` is the swap point if/when we add a banner in `render.rs`.