---
id: t-c-watch-modal-rs-dropdown
level: task
title: "T-C: watch_modal.rs — dropdown + dynamic config form state & render"
short_code: "ARAWN-T-0423"
created_at: 2026-05-26T17:28:05.324720+00:00
updated_at: 2026-05-26T18:39:40.091033+00:00
parent: ARAWN-I-0058
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0058
---

# T-C: watch_modal.rs — dropdown + dynamic config form state & render

## Parent Initiative

[[ARAWN-I-0058]]

## Objective

Build the modal as a self-contained state machine + renderer in
`arawn-tui/src/watch_modal.rs`, mirroring `todo_modal.rs` (owns its own state,
key handling outcomes, and rendering — not the generic single-choice
`ModalState`/oneshot). This task is pure UI state + render with unit tests; the
event-loop/RPC wiring is [[ARAWN-T-0424]].

## Type
Feature — `arawn-tui` (new module).

## Technical Approach

1. **State** (`WatchModalState`):
   - Two stages: `PickTemplate` (selectable list of `(template_name, description)`)
     and `FillForm` (the chosen template's fields).
   - `FillForm` holds: the template name, a `Vec<FieldState>` built from the
     `ParamSpec` list, a required `feed_id` text field, and an optional/advanced
     `cadence` text field pre-filled with the template default.
   - `FieldState` carries the `ParamSpec` + current edit value + per-field
     validity (required-empty, int-parse, etc.).
2. **Field widgets by `ParamKind`**: text/path → text input; bool → toggle;
   int → numeric text; enum → inline selector; list → text that splits to a JSON
   array. Defaults pre-filled from the spec.
3. **`handle_key` → outcome enum** (like `TodoOutcome`): `Cancel`,
   `PickTemplate(name)`, `SubmitSpec { template, feed_id, params: Value, cadence: Option<String> }`,
   and field-edit transitions. No I/O here — the outcome is returned for T-D to
   dispatch.
4. **Submit assembly**: build the params `Value` so it matches what
   `parse_watch_args` produces (so the server path is identical); cadence only
   included when changed from the default.
5. **Render** (`render_watch_modal`): centered overlay via `centered_rect` +
   `Clear`; stage-1 list; stage-2 form with labels, required markers, the
   advanced cadence affordance, a validation/error line, and a key-hints footer.
6. **Advanced toggle**: cadence hidden behind an "advanced" affordance,
   pre-filled with the default.

## Acceptance Criteria

## Acceptance Criteria

- [x] `watch_modal.rs` exists with `WatchModalState`, `WatchOutcome`, and
      `render_watch_modal`. Registered in `lib.rs`.
- [x] Stage 1 lists templates (sorted) and navigates with ↑↓; Enter emits
      `TemplatePicked(name)` for the event loop to fetch schema + `enter_form`.
- [x] Stage 2 renders one field per `FeedParamSpecDto` (all kinds), defaults
      pre-filled; required `feed_id` first; cadence last under an "— advanced —"
      separator, pre-filled with the template default.
- [x] Submit blocked on empty required field or invalid typed field; the reason
      is exposed via `last_error` and rendered.
- [x] Valid submit yields `WatchOutcome::Submit { template, feed_id, params,
      cadence }` with params coerced to match `parse_watch_args` output (Since
      resolved via the now-`pub(crate)` `command::parse_since`); cadence is
      `Some` only when changed from default; empty optionals omitted.
- [x] `Esc` yields `Cancel` in both stages.
- [x] 9 pure-state unit tests: pick-nav, schema→field seeding, required-empty
      block, spaced-path payload, bool toggle, list split, int validation,
      cadence override, esc-cancel.
- [x] `cargo build -p arawn-tui` clean; `arawn-tui` watch_modal tests pass.

## Dependencies
Depends on [[ARAWN-T-0421]] (`ParamSpec`/`ParamKind` shape) for the field model.
Consumes data fetched by [[ARAWN-T-0422]] at runtime, but can be built/tested
against `ParamSpec` directly. Blocks [[ARAWN-T-0424]].

## Risk Considerations
- Keep all I/O out of this module (mirror `todo_modal.rs`): pure state + render
  keeps it unit-testable and leaves RPC concerns to T-D.
- Field-level discovery (Slack channel picker) is wired in T-D; design
  `FieldState` so a field can later be backed by a discovered list without a
  rewrite.

## Status Updates

**2026-05-26 — Implemented + tested.**
- New `crates/arawn-tui/src/watch_modal.rs`: `WatchModalState` (two-stage:
  `PickTemplate` / `FillForm`), `WatchOutcome` (None / TemplatePicked / Submit /
  Cancel), `FieldState`, `render_watch_modal`. Registered in `lib.rs`.
- `enter_form()` is the seam the event loop (T-D) calls after fetching schema:
  builds focusable rows = feed_id + params + cadence, pre-filling defaults.
- Field widgets by kind: text/path/since/int = text input; bool = `[on]/[off]`
  toggle (space/←/→); enum = `< value >` selector (←/→); list = space/comma →
  JSON array. `coerce_param` builds the submit payload; `Since` resolved via the
  now-`pub(crate)` `command::parse_since`.
- Pure state + render, no I/O (mirrors `todo_modal.rs`). 9 unit tests pass.

**Design notes:** cadence is a regular optional field rendered last under an
"— advanced —" separator (no separate toggle key — simpler, and text fields
capture all chars so a hotkey toggle would conflict). Field-level discovery
picker (Slack channel etc.) deferred to T-D as planned; `FieldState` can later
carry a discovered choice list without a rewrite.