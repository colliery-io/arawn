---
id: t-b-remove-write-target-lens
level: task
title: "T-B: Remove write-target + /lens switch; /lens becomes create/list/show"
short_code: "ARAWN-T-0437"
created_at: 2026-05-27T20:18:49.413618+00:00
updated_at: 2026-05-27T20:18:49.413618+00:00
parent: ARAWN-I-0061
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0061
---

# T-B: Remove write-target + /lens switch; /lens becomes create/list/show

## Parent Initiative
[[ARAWN-I-0061]]

## Objective
Lenses are standing, memory-aware signal extractors — not places you switch into.
Remove the write-target mechanic retained by I-0060: the `/lens switch`
subcommand, the status-bar `✎` write-target indicator, and the
SessionLens-as-write-target plumbing. `/lens` keeps `create` / `list` / `show`.

## Technical Approach
- **Slash command**: drop `/lens switch` (`crates/arawn-tui/src/command.rs` lens
  subcommands); keep `create`/`list`/`show`. Update `/help`.
- **Event loop**: remove the switch handler that relists sessions + clears chat
  (`crates/arawn-tui/src/event_loop/mod.rs:511-531`).
- **Status bar**: remove the `✎ <lens>` write-target span
  (`crates/arawn-tui/src/render/status_bar.rs:36-45`).
- **Engine/service**: remove `SessionLens` as a write target; remove the
  `lens_switch` tool (`tools/lens/switch.rs`) and its registration.
- **Sessions**: a session is no longer "in a lens." Confirm session creation no
  longer requires a lens; default writes go to global memory (T-A).

## Acceptance Criteria
- [ ] `/lens switch` no longer exists; `/lens create|list|show` still work.
- [ ] Status bar shows no write-target/`✎` indicator.
- [ ] `lens_switch` tool and SessionLens-write-target plumbing removed; build clean.
- [ ] No code path requires choosing/switching a lens to write.
- [ ] TUI snapshots regenerated; `arawn-tui` tests + `angreal check workspace` green.

## Dependencies
Depends on [[ARAWN-T-0436]] (memory global, so writes have a home without a lens).
Pairs with [[ARAWN-T-0438]].

## Status Updates
*To be added during implementation*
