---
id: t-f-field-level-discovery-picker
level: task
title: "T-F: Field-level discovery picker in the /watch modal"
short_code: "ARAWN-T-0426"
created_at: 2026-05-26T18:54:00.629069+00:00
updated_at: 2026-05-26T19:51:00.981013+00:00
parent: ARAWN-I-0058
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

Decisions locked with the user (2026-05-26):
- **Schema hint** — add `discoverable: bool` to `ParamSpec` (+ `FeedParamSpecDto`),
  set `true` on the discovery-backed params. The modal reads the flag; no
  runtime guessing.
- **Transient sub-screen** — focusing a discoverable field + Enter opens a
  stage-1-style selectable list; choosing returns to the form with the value
  filled.
- **Lazy fetch** — `feed_discover` is called the first time the field's picker
  is opened (never blocks form open); rows cached on the field thereafter.

1. **Mark discoverable**: add `discoverable: bool` (serde default false) to
   `ParamSpec` + a `.discoverable()` builder; set it on `slack/channel-archive`
   `channel`, `jira/project-tracker` `project`, `confluence/space-archive`
   `space_key`. Mirror the flag onto `FeedParamSpecDto` and `param_spec_to_dto`.
2. **Modal state**: `FieldState` gains `discoverable` + `choices: Option<Vec<…>>`
   (None = not fetched). A `picker: Option<PickerState>` on `WatchModalState`
   holds the active sub-screen (field index + selection).
3. **Flow**: Enter on a discoverable field → if choices cached, open the picker;
   else emit `WatchOutcome::Discover { template, field_key }`. The event loop
   calls `feed_discover`, then `state.set_field_choices(key, rows)` which caches
   and opens the picker (empty/error → caches empty + stays in form so the user
   types it = free-text fallback). Picker: ↑↓ select, Enter fills the field's
   value from the row's param + closes, Esc closes back to the form.
4. **Keep it pure**: the modal holds rows + selection; `event_loop/watch.rs` does
   the `feed_discover` I/O (mirrors how `enter_form` feeds schema in).

## Acceptance Criteria

## Acceptance Criteria

- [x] For a discovery-backed template the field shows `↵ choose…` and opens a
      pick-list (label + hint); selecting fills the field's value from the row's
      `params[field_key]`.
- [x] Non-discoverable fields unchanged (typed text); discoverable flag carried
      end to end (ParamSpec → FeedParamSpecDto → modal).
- [x] Discovery failure / empty degrades to free text (`set_field_choices([])`
      → no picker + "type the value instead" hint; field still editable).
- [x] Submit payload shape unchanged — picker just sets a field value, then the
      same `build_submit` path runs.
- [x] Tests: `enter_on_discoverable_field_requests_discovery_then_picks`,
      `cached_choices_open_picker_without_refetch`, `empty_discovery_falls_back_to_free_text`,
      `esc_in_picker_returns_to_form_without_cancelling`, + `discovery_choices`
      mapping tests in `event_loop/watch.rs`. `angreal check workspace` clean;
      29 arawn-tui watch tests pass; `angreal docs build` clean.

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

**2026-05-26 — Implemented + tested.**
- `arawn-feeds`: `ParamSpec.discoverable` (serde default false) + `.discoverable()`
  builder; set on slack/channel-archive `channel`, jira/project-tracker `project`,
  confluence/space-archive `space_key`.
- `arawn-service`: `FeedParamSpecDto.discoverable` (serde default); mapped in
  `param_spec_to_dto`.
- `arawn-tui/watch_modal.rs`: `DiscoveryChoice`, `PickerState`,
  `FieldState.choices`, `WatchModalState.picker`, `WatchOutcome::Discover`,
  `set_field_choices`, `open_picker`, `handle_picker_key`, and the picker
  sub-screen render. Discoverable empty fields show `↵ choose…`.
- `arawn-tui/event_loop/watch.rs`: handles `Discover` → `feed_discover` →
  `discovery_choices(value, field_key)` → `set_field_choices`.
- Docs: create-a-feed.md notes the in-field picker + free-text fallback.
- Tests green (29 watch), workspace + docs build clean.

Closes REQ-007. With this, all of I-0058's planned slices (T-A..T-F) are done.