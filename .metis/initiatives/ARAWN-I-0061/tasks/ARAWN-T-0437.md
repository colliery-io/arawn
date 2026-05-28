---
id: t-b-remove-write-target-lens
level: task
title: "T-B: Remove write-target + /lens switch; /lens becomes create/list/show"
short_code: "ARAWN-T-0437"
created_at: 2026-05-27T20:18:49.413618+00:00
updated_at: 2026-05-28T03:05:34.494053+00:00
parent: ARAWN-I-0061
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

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

**2026-05-27 — Done.** Removed the user-facing write-target/switch surface AND
fully deleted the session-promotion stack (user steered: "lenses aren't switched
into" + "there is no promotion into a lens").

Switch removal:
- `/lens switch` command + `LensSwitch` variant + event-loop handler + actions.rs
  reference dropped (`command.rs`, `event_loop/mod.rs`, `app/actions.rs`); status
  bar `✎ <lens>` span removed (`render/status_bar.rs`).
- `lens_switch` tool deleted (`tools/lens/switch.rs`) + mod/exports
  (`lens/mod.rs`, `tools/mod.rs`, `engine/lib.rs`) + main.rs registration; its
  two unit tests removed; `query_engine` lens-visibility tests retargeted to
  `lens_show`.

Promotion removal (full stack, no orphans):
- Tool: `LensPromoteTool` (`tools/lens/promote.rs`) + mod/exports + registration.
- TUI: `/promote` CommandInfo, `CommandResult::PromoteSession`, "promote" arm,
  event-loop handler, actions.rs reference.
- RPC + Service: ws_server `promote_session` route + methods list entry;
  `arawn-service` trait method + `PromotionResult` type (lib.rs + types.rs);
  `LocalService::promote_session` impl + `promote_session_inner`.
- Storage: `Store::promote_session`, `promote_session_metadata`,
  `move_session_jsonl`, `copy_dir_contents`, plus the orphaned
  `update_session_lens_name` (switch persistence). `SessionStore::update_lens_id`
  and `update_lens_name` setters removed along with their two tests.
- Integration tests: `engine_persistence::scratch_session_promotion_preserves_messages`
  and `local_service::{promote_scratch_session_to_lens, promote_non_scratch_session_fails}`
  removed; dead fixture `scratch_session` + dead `JsonlMessageStore` import cleaned.

No tombstone comments left behind.

Verification: `angreal check workspace` exit 0. Storage 72/0, engine lib 695/0
(plus 2 shell-harness tests that only fail under the macOS sandbox — unrelated to
this change), tui 248/0 (all status-bar/dashboard/idle snapshots regenerated +
accepted), arawn-tests local_service 4/0 + engine_persistence 14/0.