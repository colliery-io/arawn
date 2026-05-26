---
id: t-e-docs-uat-for-watch-modal
level: task
title: "T-E: Docs + UAT for /watch modal registration"
short_code: "ARAWN-T-0425"
created_at: 2026-05-26T17:28:07.575295+00:00
updated_at: 2026-05-26T18:55:04.553339+00:00
parent: ARAWN-I-0058
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0058
---

# T-E: Docs + UAT for /watch modal registration

## Parent Initiative

[[ARAWN-I-0058]]

## Objective

Document the new modal flow and close the initiative with a test that proves
register-via-modal works end to end for both a free-form template
(`filesystem/folder`) and a discovery-backed one.

## Type
Documentation + Testing — `docs/`, `arawn-tui` / `arawn-tests`.

## Technical Approach

1. **Docs**: update the feed/watch docs (`docs/src/reference/feed-templates.md`
   and wherever `/watch` is described) to lead with the modal flow — `/watch`
   (no args) → pick template → fill form → submit — and keep the one-line
   command documented as the power-user path. Note the param-schema mechanism
   briefly so future template authors know they must implement `param_schema()`.
2. **Test**: add a TUI-level test driving the modal state machine through a full
   register for `filesystem/folder` (fill `root` + `feed_id`, submit, assert the
   `feed_register` payload) and one discovery-backed template (assert the
   discovered choice fills the field). Prefer a deterministic state-machine /
   harness test over a real-LLM UAT, since this is mechanical UI, not agent
   behavior — only add a `angreal test uat` scenario if a meaningful agent-driven
   path exists.
3. **Docs build**: `angreal docs build` clean (no broken anchors).

## Acceptance Criteria

## Acceptance Criteria

- [x] Watch/feed docs describe the modal flow and still document the one-line
      command; param-schema requirement for template authors is noted
      (`docs/src/how-to/create-a-feed.md` + `reference/slash-commands.md`).
- [x] A test registers `filesystem/folder` through the modal state machine and
      asserts the resulting payload — and that it deserializes into the real
      `FeedRegisterSpec` (incl. a spaced path typed with no quoting):
      `submit_is_wire_compatible_with_feed_register`.
- [~] Discovery-backed field picker test — **moved to [[ARAWN-T-0426]]** (the
      picker itself was deferred from T-D). T-E covers the free-form path
      end-to-end; the discovery path lands with T-F.
- [x] `angreal docs build` passes; `angreal check workspace` + `arawn-tui` tests
      pass (10 watch_modal tests).
- [x] Initiative exit criteria reviewed: core flow done; REQ-007 tracked as
      [[ARAWN-T-0426]] so I-0058 stays open until that lands (or is explicitly
      descoped).

## Test Cases

### TC-001: Register filesystem feed via modal
- **Preconditions**: T-A…T-D landed; modal reachable.
- **Steps**: open `/watch` → pick `filesystem/folder` → type a `root` containing
  a space → type `feed_id` → submit.
- **Expected**: `feed_register` called with `root` intact (no quoting), correct
  feed_id, default cadence; feed listed in `/feeds`.

### TC-002: Discovery-backed field
- **Preconditions**: a discovery-backed template available in test harness.
- **Steps**: open `/watch` → pick it → field offers discovered choices → select
  one → fill feed_id → submit.
- **Expected**: selected choice's params land in the submitted spec.

## Dependencies
Depends on [[ARAWN-T-0424]] (full flow must work). Final task of the initiative.

## Risk Considerations
- A real-LLM UAT may be overkill for a mechanical UI; default to a deterministic
  harness test and document the rationale (consistent with prior feedback to
  solve at the right layer).

## Status Updates

**2026-05-26 — Docs + test done.**
- Docs: `create-a-feed.md` now leads `/watch` with the guided form (pick →
  fill → Enter; feed-id always asked; cadence advanced; spaced paths need no
  quoting), keeps the one-line form as the power-user path, and adds a
  template-author note about the required `param_schema()`. `slash-commands.md`
  `/watch` entry updated. `angreal docs build` clean.
- Test: `submit_is_wire_compatible_with_feed_register` drives the full modal
  flow (pick → enter_form with the server's filesystem schema → fill → submit)
  and proves the payload deserializes into the real `arawn_service::FeedRegisterSpec`,
  with a spaced path intact and unchanged cadence not overridden.
- Chose a deterministic state-machine test over a real-LLM UAT: this is
  mechanical UI, not agent behavior (right-layer testing).
- Discovery-picker test deferred with the feature to [[ARAWN-T-0426]].