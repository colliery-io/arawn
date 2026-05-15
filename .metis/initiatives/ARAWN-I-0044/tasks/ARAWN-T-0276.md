---
id: approval-tiering-session-scoped
level: task
title: "Approval tiering — session-scoped Always allowlists + audit log"
short_code: "ARAWN-T-0276"
created_at: 2026-05-15T14:12:58.452098+00:00
updated_at: 2026-05-15T14:12:58.452098+00:00
parent: ARAWN-I-0044
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0044
---

# Approval tiering with session allowlist

## Tier
Tier 1 — touches the TUI; small but has coordination cost with the existing `permissions/` module.

## Reference
`/tmp/openhuman/src/openhuman/approval/`. Pre-execution hook with Always / Once / Deny tiers, session-scoped allowlists, audit log.

## Goal
Tools flagged as sensitive (shell, file_write, file_edit, sensitive env reads) prompt the user before execution. Three responses: Allow once / Allow for session / Deny. "Allow for session" is keyed by `(tool, normalised-args-shape)`. Every decision goes to an audit log.

## Acceptance
- New `crates/arawn-engine/src/approval/{mod,allowlist,audit}.rs`.
- Integrates with existing `permissions/` rather than replacing it. Permissions decide *whether* approval is needed; approval handles the interaction.
- Session allowlist keyed by tool + argument shape (not exact args — paths normalised, env names matched).
- Audit log appended to data dir; one line per decision (tool, shape, verdict, timestamp, session id).
- TUI prompt UI for the three tiers; non-TUI callers fail closed with a clear message.
- Tests cover allowlist hit/miss, shape normalisation, audit log invariants.

## Out of scope
Persistent (cross-session) allowlists — those are a follow-up. Session-scoped only here.
