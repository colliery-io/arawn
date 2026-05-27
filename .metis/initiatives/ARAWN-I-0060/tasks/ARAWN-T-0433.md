---
id: t-d-decouple-persona-from-active
level: task
title: "T-D: Decouple persona from active lens (default assistant)"
short_code: "ARAWN-T-0433"
created_at: 2026-05-27T02:35:57.015227+00:00
updated_at: 2026-05-27T14:28:07.214395+00:00
parent: ARAWN-I-0060
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0060
---

# T-D: Decouple persona from active lens (default assistant)

## Parent Initiative

[[ARAWN-I-0060]]

## Objective

A roaming chat has no single lens, so it can't take its persona from one. Default
the chat persona to `assistant`; stop flipping `identity_profile` per active
lens.

## Type
Feature — `arawn-engine/system_prompt.rs`, `query_engine.rs`,
`arawn/src/local_service` session-context build.

## Technical Approach

- `build_session_context` / `PromptContext` stop sourcing `identity_profile` from
  the active lens; default to `IdentityProfile::Assistant`.
- The lens's `identity_profile` field stays on the record (still meaningful for
  *writing*/extraction into that lens, and for AWEN later) but no longer drives
  the chat system prompt.
- Decide the `coding` story: a `coding` lens no longer auto-switches the chat
  persona. If a coding persona is still wanted, it becomes an explicit session
  choice (out of scope here — default assistant; note the follow-up).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Chat system prompt uses `assistant` regardless of any current/target lens.
- [ ] No session-build path reads `lens.identity_profile` to pick the chat
      persona.
- [ ] `lens.identity_profile` remains stored/settable (no schema change).
- [ ] `angreal check workspace` + system-prompt/snapshot tests updated + green.

## Dependencies
Depends on [[ARAWN-T-0432]] (active lens demoted). Independent of T-E/T-F.

## Status Updates

**2026-05-27 — Done** (commit `5517c2a`). Both PromptContext build sites
(`startup/engine.rs`, `local_service/mod.rs`) now set
`identity_profile = IdentityProfile::Assistant` instead of `lens.identity_profile`.
The lens column is untouched (still stored/settable; matters for writes + AWEN)
but no longer drives the chat system prompt. `arawn` lib 57/0; build clean.
Coding-persona-on-demand left as a future follow-up.