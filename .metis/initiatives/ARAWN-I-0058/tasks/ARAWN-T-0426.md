---
id: t-f-field-level-discovery-picker
level: task
title: "T-F: Field-level discovery picker in the /watch modal"
short_code: "ARAWN-T-0426"
created_at: 2026-05-26T18:54:00.629069+00:00
updated_at: 2026-05-26T18:54:00.629069+00:00
parent: ARAWN-I-0058
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0058
---

# T-F: Field-level discovery picker in the /watch modal

## Parent Initiative

[[ARAWN-I-0058]]

## Objective

Make a discoverable param in the `/watch` modal offer the provider's choices
inline (REQ-007), instead of requiring the user to type a raw id. E.g. the
`channel` field on `slack/channel-archive` presents the workspace's channels to
pick from; `jira/project-tracker`'s `project` and `confluence/space-archive`'s
`space_key` likewise. Split out of [[ARAWN-T-0424]] — the modal shipped with
typed-text fields for these; this is the polish that closes REQ-007.

## Type
Feature — `arawn-tui` (watch modal + event-loop glue). `feed_discover` RPC
already exists.

## Technical Approach

1. **Which fields are discoverable**: a template's `discover()` returns rows
   whose `params` object names the key(s) it resolves (e.g. `{"channel": "C…"}`).
   Map "this param key is discoverable for this template" — either by checking
   the discover rows' param keys at form-build time, or by adding a
   `discoverable: bool` hint to `ParamSpec`/`FeedParamSpecDto` (preferred: the
   template already knows). Decide at task start.
2. **Fetch**: when entering the form (or lazily when focusing such a field),
   call `feed_discover(template)`; cache rows on the `FieldState`.
3. **Widget**: a discoverable field renders as a selectable list (reuse the
   stage-1 list affordance) showing `label` (+ `hint`); selecting fills the
   field's value from the row's `params`. Allow falling back to free text when
   discovery returns nothing / errors (offline, no perms).
4. **Keep it pure where possible**: the modal holds the rows + selection;
   `event_loop/watch.rs` does the `feed_discover` I/O and feeds rows in (mirror
   how schema is fed via `enter_form`).

## Acceptance Criteria

- [ ] For a discovery-backed template, the relevant field offers the discovered
      choices (label + hint) and selecting one fills the submitted param.
- [ ] Non-discoverable fields are unchanged (typed text).
- [ ] Discovery failure (offline / no perms / empty) degrades to a free-text
      field, not a dead end.
- [ ] Submit payload unchanged in shape (still a valid `FeedRegisterSpec`).
- [ ] Unit tests for the field's select-fills-param behavior; `angreal check
      workspace` + `arawn-tui` tests pass.

## Dependencies
Builds on [[ARAWN-T-0423]] (modal) and [[ARAWN-T-0424]] (event-loop glue +
`feed_templates`). `feed_discover` already exists.

## Risk Considerations
- Discovery is a network call — keep it lazy/cached and never block the form on
  it (fall back to text).
- If adding a `discoverable` hint to the schema, that touches `ParamSpec` +
  the service DTO + the per-template declarations — weigh against detecting it
  from `discover()` rows at runtime.

## Status Updates

*To be added during implementation*