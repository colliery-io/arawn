---
id: t-h-first-run-no-model-guidance-in
level: task
title: "T-H: First-run no-model guidance in the TUI"
short_code: "ARAWN-T-0443"
created_at: 2026-05-27T20:18:49.413618+00:00
updated_at: 2026-05-28T15:26:56.304809+00:00
parent: ARAWN-I-0061
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0061
---

# T-H: First-run no-model guidance in the TUI

## Parent Initiative
[[ARAWN-I-0061]]

## Objective
A user who launches before configuring an LLM provider sees a silent `no model`
in the status bar and a friendly hero, then types a message and gets a failure.
Surface actionable guidance instead of a dead-end.

## Scope
- `crates/arawn-tui/src/render/status_bar.rs:20-25` ("no model") — when no model
  is configured, point the user at the fix (run `arawn doctor` / edit
  `~/.arawn/arawn.toml`), e.g. via the idle hero, a toast on first input, or
  inline status text.
- Choose the simplest surface that's actually seen on first run (hero text is the
  most visible). Avoid over-building.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] With no model configured, the TUI shows actionable guidance (mentions
      `arawn doctor` and/or `arawn.toml`), not just "no model."
- [ ] Guidance is unobtrusive once a model is configured.
- [ ] Snapshots updated if the hero/status changes; `arawn-tui` tests green.

## Dependencies
Independent.

## Status Updates

**2026-05-28 — Done.** Hero now surfaces actionable first-run guidance when no
LLM is configured.

- `render_idle_hero` (`render/chat.rs`) gains a `no_model: bool` arg; the
  caller in `render_chat` passes `app.model_name.is_empty()`.
- When `no_model` is true the hero inserts three lines under the welcome
  block: `No LLM provider configured.` (yellow), `Run `arawn doctor` to
  diagnose.`, `Edit `~/.arawn/arawn.toml` to set one.` Sized to fit the
  existing 56-char hero rect; no layout change.
- New snapshot `snapshot_idle_hero_no_model_configured` exercises the
  no-model path; the existing `snapshot_idle_hero` was tightened to set
  `model_name = "test-model"` so it represents the configured baseline
  cleanly. Both snapshots regenerated and accepted.

Verification: `arawn-tui` 254/0; `angreal check workspace` exit 0.