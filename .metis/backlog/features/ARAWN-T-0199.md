---
id: surface-hot-reload-status-plugins
level: task
title: "Surface hot-reload status (plugins, config) in TUI"
short_code: "ARAWN-T-0199"
created_at: 2026-05-02T00:00:00+00:00
updated_at: 2026-05-02T00:00:00+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


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

*To be added during implementation*
