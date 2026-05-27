---
id: watch-modal-schema-driven-feed
level: initiative
title: "/watch modal — schema-driven feed registration form"
short_code: "ARAWN-I-0058"
created_at: 2026-05-26T17:14:31.449214+00:00
updated_at: 2026-05-27T01:39:51.171282+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: watch-modal-schema-driven-feed
---

# /watch modal — schema-driven feed registration form

## Context

Registering a continual data feed today is a single-line command:
`/watch <provider/template> <feed_id> [key=value ...]`. It works, but the user
must already know the canonical template name, which params a template takes,
their types, and their defaults — none of which the TUI surfaces. The recent
filesystem-feed work (ARAWN-I-0057) exposed two sharp edges: a path with spaces
needs careful quoting (fixed in `tokenize_kv`), and there's no way to discover
that `filesystem/folder` even takes a `root`/`recursive`/`include`/`exclude`
shape without reading the docs.

The fix is a guided modal: `/watch` with no args opens a form — pick a template
type from a dropdown, then fill a config form whose fields are derived from the
template's own parameter schema. This removes the "memorize the grammar" tax and
makes every feed self-documenting at the point of registration.

The blocker for a *generic* form is that `FeedTemplate` exposes no
machine-readable parameter schema today — only `validate()`, `defaults()`, and
`discover()` (the latter only for templates backed by an external picker like
Slack channels). This initiative adds that schema and drives the form from it.

## Goals & Non-Goals

**Goals:**
- `/watch` with no args opens a modal: template-type dropdown → per-template
  config form → submit, calling the existing `feed_register` RPC.
- Add a machine-readable `param_schema()` as a **required** method on the
  `FeedTemplate` trait (no default impl) and implement it for **all** templates,
  so every field is labeled, typed, defaulted, and marked required/optional.
  Making it required means the compiler refuses any feed template that hasn't
  described its config — no silent gaps, now or for future templates.
- Feed-id is an explicit, required field the user must fill (no auto-generate).
- Cadence is an optional "advanced" field, pre-filled with the template's
  default cadence; editing it sets the `@cadence` override.
- Discovery-backed templates (Slack, Jira, …) integrate their `discover()`
  picker into the relevant field rather than asking for a raw id.
- The existing one-line `/watch <template> <id> k=v` command keeps working
  unchanged — the modal is additive.

**Non-Goals:**
- No change to the feed runtime, cron, scan/diff, or projection layers.
- No new feed templates.
- No multi-feed batch registration.
- Not replacing `/watch list` discovery text (it can stay or be folded in
  later; not required here).

## Requirements

### Functional
- REQ-001: Typing `/watch` (no args) opens the modal; `Esc` cancels with no
  side effects.
- REQ-002: Stage 1 is a scrollable/selectable list of all registered template
  names with a one-line description each.
- REQ-003: Stage 2 renders one input per declared param: text, bool (toggle),
  integer, path, and enum (select) field kinds, pre-filled with declared
  defaults; required fields are marked and block submit when empty.
- REQ-004: A required `feed_id` field is always present.
- REQ-005: An optional cadence field is present under an "advanced" affordance,
  pre-filled with the template default; a non-default value becomes the
  `@cadence` override in the submitted spec.
- REQ-006: Submit builds a `WatchSpec`-equivalent payload and calls the same
  `feed_register` RPC path the text command uses; success/error is surfaced in
  the modal (errors keep the form open).
- REQ-007: For a param whose template supports `discover()`, the field offers
  the discovered choices (e.g. pick a Slack channel) instead of a raw text id.

### Non-Functional
- NFR-001: `param_schema()` is a **required** trait method (no default impl).
  Every template must declare its schema; the build fails otherwise. There is no
  free-text fallback — the modal always renders from a real schema. Because the
  trait change and all template impls land together, the migration is atomic
  rather than incremental (T-A and T-B are effectively one landing).
- NFR-002: Schema declarations live next to each template's `validate()` so the
  two stay in sync; validation remains the source of truth on the server. A test
  asserts every registered template returns a non-empty, self-consistent schema.

## Use Cases

### Register a filesystem feed via the modal
- **Actor**: User watching a local notes folder.
- **Scenario**: Types `/watch` → picks `filesystem/folder` → form shows
  `root` (path, required), `recursive` (toggle, default on), `include` /
  `exclude` (lists, defaulted), `feed_id` (required), cadence (advanced,
  pre-filled `*/15 * * * *`). Fills `root` and `feed_id`, hits submit.
- **Expected Outcome**: Feed registered; a path with spaces just works (typed
  into a field, no quoting). Confirmation shows template, id, cadence.

### Register a Slack channel feed
- **Actor**: User archiving a Slack channel.
- **Scenario**: `/watch` → `slack/channel-archive` → the `channel` field offers
  the discovered channel list to pick from; fills `feed_id`; submit.
- **Expected Outcome**: Registered without the user knowing the channel's `C…`
  id.

## Architecture

### Overview
A new custom-state-machine modal in `arawn-tui` (mirroring `todo_modal.rs`,
which owns its own state, key handling, and RPC dispatch — the generic
single-choice `ModalState`/oneshot is too thin for a multi-field form). The
modal is fed by a new typed param-schema surfaced from `arawn-feeds` through the
service layer.

### Components
- `arawn-feeds`: new `ParamSpec`/`ParamKind` types + a **required**
  `FeedTemplate::param_schema()` method, implemented by every template.
- `arawn-service` + RPC: a way to fetch a template's schema (extend the existing
  `feed_discover` response, or a sibling `feed_schema` method) and the existing
  `feed_register`.
- `arawn-tui`: `watch_modal.rs` (state + render) and `event_loop/watch.rs`
  (key handling + submit), wired into the event loop and `App` state next to the
  todo overlay.

### Sequence
`/watch` → event loop fetches template list (+ schemas) → modal stage 1 →
on template pick, ensure schema present (and `discover()` choices if any) →
stage 2 form → submit → `feed_register` RPC → close + confirm, or keep open on
validation error.

## Detailed Design

Decisions locked with the user (2026-05-26):
- **Full schema migration** — populate `param_schema()` for every template, not
  just filesystem.
- **`param_schema()` is required, not optional** — no default impl, no free-text
  fallback. The compiler enforces that every feed describes its config.
- **Always ask for feed-id** — explicit required field, no auto-generation.
- **Cadence = advanced optional field** — pre-filled with the template default;
  override only when changed.

`ParamKind` (initial set): `Text`, `Bool`, `Int`, `Path`, `Enum(Vec<String>)`,
`List` (comma/space-separated → JSON array). `ParamSpec { key, label, kind,
required, default: Option<Value>, help }`. The form maps each spec to a widget;
on submit it assembles a JSON object matching what `parse_watch_args` would have
produced, so the server path is identical.

Field-level `discover()` integration: when a `ParamSpec` is flagged as
discoverable (or the template advertises discovery for that key), the field is a
selectable list sourced from `feed_discover` rather than free text.

## Alternatives Considered

- **No schema, free-text params field (Option A).** Modal with a template
  dropdown + a single `key=value` textarea. Less work, but the filesystem form
  stays "type `root=…`" — fails the core UX goal. Rejected entirely; with a
  required `param_schema()` there is no schema-absent case to fall back to.
- **Optional `param_schema()` with an empty default + free-text fallback.**
  Backward-compatible and incremental, but invites silent gaps: a new template
  could ship with no config description and quietly degrade to free text. Making
  the method required moves that failure to compile time. Rejected per the
  user's call ("make it required — no reason not to").
- **Hybrid — schema for filesystem only (Option C).** Ship the form for the one
  template that motivated this, others fall back to free text. Lower up-front
  cost, but leaves the discovery gap for every other feed. User chose the full
  migration instead.
- **Reuse the generic `ModalState`/oneshot.** Designed for a single choice;
  shoehorning a multi-field, multi-stage form into it is more friction than a
  purpose-built state machine like `todo_modal.rs`.

## Implementation Plan

Proposed task breakdown (decompose after design approval):
- **T-A — Param schema types + required trait method + all template impls.** Add
  `ParamSpec`/`ParamKind` and a **required** `FeedTemplate::param_schema()`, and
  implement it for every template in the same change (no default impl means the
  workspace won't compile until all templates are covered — so this is one
  landing, not two). Schemas live next to each template's `validate()`. Unit-test
  the types; add a test asserting every registered template returns a non-empty,
  self-consistent schema.
- **T-B — Surface schema over the service/RPC.** Add a `feed_schema` RPC (kept
  separate from `feed_discover`) so the TUI can fetch a template's schema + any
  discoverable fields; wire the `ws_client` method.
- **T-C — Watch modal state + render.** `watch_modal.rs`: stage-1 dropdown,
  stage-2 dynamic form (all field kinds), feed-id field, advanced cadence
  field. Pure-state unit tests.
- **T-D — Event-loop wiring + submit.** `/watch` (no args) opens the modal; key
  routing; submit builds the spec and calls `feed_register`; success/error
  surfacing. Field-level discovery picker integration.
- **T-E — Docs + UAT.** Update `feed-templates.md` / watch docs to describe the
  modal; add a TUI-level test (or UAT scenario) covering register-via-modal for
  filesystem and one discovery-backed template.

## Open Questions
- ~~Extend `feed_discover` vs. add a sibling `feed_schema` RPC?~~ **Resolved:**
  add a separate `feed_schema` RPC (T-B) to keep discovery and schema concerns
  distinct.
- Does any current template have a param that's awkward under this `ParamKind`
  set (e.g. structured/nested params)? Audit during T-A; widen `ParamKind` if
  needed.