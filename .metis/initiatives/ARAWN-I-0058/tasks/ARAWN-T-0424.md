---
id: t-d-event-loop-wiring-watch-opens
level: task
title: "T-D: Event-loop wiring — /watch opens modal, submit via feed_register"
short_code: "ARAWN-T-0424"
created_at: 2026-05-26T17:28:06.717620+00:00
updated_at: 2026-05-26T18:50:32.406472+00:00
parent: ARAWN-I-0058
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0058
---

# T-D: Event-loop wiring — /watch opens modal, submit via feed_register

## Parent Initiative

[[ARAWN-I-0058]]

## Objective

Wire the [[ARAWN-T-0423]] modal into the live TUI: `/watch` with no args opens
it, keys route to it, it fetches schema via [[ARAWN-T-0422]], and a valid submit
calls the existing `feed_register` RPC and surfaces success/error. Also wire
field-level discovery (Slack channel, Jira project, …) so discoverable fields
present picked choices.

## Type
Feature — `arawn-tui` event loop + `App` state.

## Technical Approach

1. **App state**: hold `Option<WatchModalState>` next to the todo overlay
   (`app/mod.rs`); add an action/command variant so `/watch` with empty args
   opens it (the non-empty form still goes through `parse_watch_args`).
2. **Open**: on `/watch` (no args), fetch the template list (names + one-line
   descriptions) for stage 1. Lazily fetch `feed_schema(template)` (T-B) when a
   template is picked, seeding stage 2; pre-fill cadence from the returned
   default.
3. **Key routing**: when the watch overlay is `Some`, route all keys to it
   (mirror the todo-overlay branch in `event_loop/mod.rs`); add
   `event_loop/watch.rs` for outcome dispatch.
4. **Submit**: on `WatchOutcome::SubmitSpec`, build the `feed_register` payload
   (identical shape to the text path) and call `client.feed_register`. On
   success, close + confirmation message (template, id, cadence). On error, keep
   the modal open and show the server error.
5. **Field discovery**: for a discoverable field, call `feed_discover(template)`
   and present the rows as the field's choices (reusing the existing discovery
   data); selecting a row fills that field's param value.
6. **Cancel**: `Esc` closes with no side effects.

## Acceptance Criteria

## Acceptance Criteria

- [x] `/watch` with no args opens the modal (`CommandResult::FeedWatchModal`);
      `/watch <template> <id> k=v` and `/watch list …` unchanged.
- [x] Picking a template fetches its schema (`feed_schema`) and renders the form
      via `enter_form`; cadence pre-filled from the returned default.
- [x] Valid submit calls `feed_register` with the same payload shape as the text
      path; on success the modal closes and reports via `format_feed_registered`.
- [x] Server validation error keeps the modal open with the message in
      `last_error`; `Esc` cancels cleanly (no side effects).
- [ ] **DEFERRED** — field-level discovery picker (offering discovered Slack
      channels/Jira projects in-field). The modal works without it: discoverable
      params are typed as text today. Tracked as a follow-up (see Status); not a
      blocker for the modal's core value. New `feed_templates` RPC + catalog were
      added for stage 1, but the per-field discovery integration is out of this
      slice.
- [x] Key routing: a `watch_overlay.is_some()` branch routes all keys to the
      modal (mirrors the todo-overlay branch) — no leakage to chat input.
- [x] `angreal check workspace` clean; `arawn-tui` tests pass (239). Modal
      outcome logic is unit-tested in T-C; the live dispatch glue is exercised
      end-to-end in T-E.

**Scope add (needed for stage 1):** a `feed_templates` RPC + `arawn-feeds`
`template_catalog()`/`template_blurb()` (one-line descriptions, covered by a
registry-coverage test) — the old `format_known_templates` text was hardcoded
and stale, so the picker needed a real, complete source.

## Dependencies
Depends on [[ARAWN-T-0421]], [[ARAWN-T-0422]] (schema RPC), and [[ARAWN-T-0423]]
(modal state/render). Last code task before docs/UAT ([[ARAWN-T-0425]]).

## Risk Considerations
- Follow the todo-overlay pattern exactly for key routing to avoid input-focus
  bugs.
- Keep the submit payload byte-for-byte equivalent to `parse_watch_args` output
  so the server path and its tests stay authoritative.

## Status Updates

**2026-05-26 — Core wired + building.**
- `App.watch_overlay: Option<WatchModalState>` + init; render branch in
  `render/mod.rs`; key-routing branch in `event_loop/mod.rs` (mirrors todo).
- `command.rs`: `/watch` with empty args → `CommandResult::FeedWatchModal`
  (typed/`list` forms unchanged); exhaustive match arm added in `app/actions.rs`.
- New `event_loop/watch.rs`: `open_watch_modal` (fetch `feed_templates`, open
  stage 1) + `handle_watch_overlay_key` (TemplatePicked→`feed_schema`+enter_form;
  Submit→`feed_register`, close+report on ok, keep-open+last_error on err;
  Cancel→close).
- Stage-1 source: new `feed_templates` RPC backed by `arawn-feeds::template_catalog()`
  (+ `template_blurb`), replacing the stale hardcoded `format_known_templates`
  text. Registry-coverage test added.
- `angreal check workspace` clean; 239 arawn-tui tests pass.

**Deferred (follow-up):** field-level discovery picker (REQ-007) — a focused
discoverable field offering `feed_discover` choices inline. Out of this slice;
discoverable params are typed as text for now. Suggest a new task under I-0058
(e.g. T-F) rather than expanding this one. Will note in the initiative.