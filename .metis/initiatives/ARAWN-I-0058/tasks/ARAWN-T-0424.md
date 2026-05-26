---
id: t-d-event-loop-wiring-watch-opens
level: task
title: "T-D: Event-loop wiring — /watch opens modal, submit via feed_register"
short_code: "ARAWN-T-0424"
created_at: 2026-05-26T17:28:06.717620+00:00
updated_at: 2026-05-26T17:28:06.717620+00:00
parent: ARAWN-I-0058
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

- [ ] `/watch` with no args opens the modal; `/watch <template> <id> k=v` still
      works unchanged (regression).
- [ ] Picking a template fetches its schema and renders the form; cadence
      pre-filled from the template default.
- [ ] Valid submit calls `feed_register` with a payload identical to the text
      path; the new feed appears in `/feeds`.
- [ ] Server validation error keeps the modal open and shows the message;
      `Esc` cancels cleanly.
- [ ] A discoverable field (e.g. `slack/channel-archive` `channel`) offers the
      discovered choices instead of requiring a raw id.
- [ ] Key routing doesn't leak to the chat input while the modal is open.
- [ ] `angreal check workspace` + `arawn-tui` tests pass (outcome-dispatch unit
      tests where practical; full interactivity covered by T-E).

## Dependencies
Depends on [[ARAWN-T-0421]], [[ARAWN-T-0422]] (schema RPC), and [[ARAWN-T-0423]]
(modal state/render). Last code task before docs/UAT ([[ARAWN-T-0425]]).

## Risk Considerations
- Follow the todo-overlay pattern exactly for key routing to avoid input-focus
  bugs.
- Keep the submit payload byte-for-byte equivalent to `parse_watch_args` output
  so the server path and its tests stay authoritative.

## Status Updates

*To be added during implementation*