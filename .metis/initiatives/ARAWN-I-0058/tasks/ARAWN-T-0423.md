---
id: t-c-watch-modal-rs-dropdown
level: task
title: "T-C: watch_modal.rs — dropdown + dynamic config form state & render"
short_code: "ARAWN-T-0423"
created_at: 2026-05-26T17:28:05.324720+00:00
updated_at: 2026-05-26T17:28:05.324720+00:00
parent: ARAWN-I-0058
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

- [ ] `watch_modal.rs` exists with `WatchModalState`, a `WatchOutcome` enum, and
      `render_watch_modal`.
- [ ] Stage 1 lists templates and navigates with arrows; selecting one moves to
      stage 2 seeded from that template's schema.
- [ ] Stage 2 renders one field per `ParamSpec` (all six `ParamKind`s), defaults
      pre-filled; `feed_id` field present and required; cadence under advanced,
      pre-filled with default.
- [ ] Submit is blocked while any required field (incl. `feed_id`) is empty or a
      typed field is invalid; the modal exposes the blocking reason for render.
- [ ] On valid submit, the outcome carries a `params` Value equal to what the
      equivalent `/watch … k=v` line would produce; cadence present only when
      changed.
- [ ] `Esc` yields `Cancel`.
- [ ] Pure-state unit tests cover: schema→fields seeding, required-empty block,
      int/list parsing, cadence-default vs override, and submit payload shape.
- [ ] `angreal check workspace` + `arawn-tui` tests pass.

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

*To be added during implementation*