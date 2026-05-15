---
id: approval-tiering-session-scoped
level: task
title: "Approval tiering — session-scoped Always allowlists + audit log"
short_code: "ARAWN-T-0276"
created_at: 2026-05-15T14:12:58.452098+00:00
updated_at: 2026-05-15T18:34:27.521766+00:00
parent: ARAWN-I-0044
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Status Updates

**2026-05-15 — implementation landed.**

- New `crates/arawn-engine/src/approval/` module:
  - `allowlist.rs` — `ArgShape` normalisation + `SessionAllowlist` keyed by `(tool_name, ArgShape)`.
    - `shell` / `Bash` → `shell:<command>` (literal command; distinct commands are distinct grants).
    - `file_write` / `file_edit` → `file:<parent-dir>` with `$HOME` folded to `~` (one approval covers files in the same directory).
    - `safe_env` → `env:<name>`.
    - Unrecognised tool → `<tool>:*` wildcard (matches any input).
    - Malformed JSON also falls back to wildcard.
  - `audit.rs` — `ApprovalAudit` writes JSONL records to `<data_dir>/approval-audit.jsonl`. `ApprovalTier { AllowOnce | AllowForSession | Deny | FailedClosed }`. `Disabled` variant for tests / no-data-dir.
  - `mod.rs` — re-exports.
- `permissions/checker.rs` refactored:
  - `SessionGrants` internally delegates to `SessionAllowlist`. The legacy zero-arg `grant(tool)` / `is_granted(tool)` API maps to the wildcard shape `"<tool>:*"`; new shape-aware methods `grant_shape(tool, shape)` / `is_granted_shape(tool, shape)` honour exact matches AND fall back to the wildcard for backwards-compat.
  - The `Ask` decision branch now computes an `ArgShape` from the tool input, populates the allowlist with the exact shape on "Allow For Session", and appends an `AuditRecord` for every prompt outcome (including `FailedClosed` when no prompter is wired).
  - `PermissionChecker` gained `approval_audit: Option<Arc<ApprovalAudit>>` + `with_approval_audit(...)`. Production wiring (LocalService) can pass an audit at construction; tests omit it and inherit `Disabled`.
- Tests added:
  - 11 in `approval::allowlist::tests` covering per-tool shape extraction, distinct/equal cases, allowlist grant/clear semantics.
  - 4 in `approval::audit::tests` covering disabled/enabled round-trip, parent-dir creation, missing-file read.
  - 2 e2e in `permissions::checker::tests` — `shape_aware_grant_only_allows_matching_shape` proves the new granularity (granting `shell:ls` does not auto-allow `shell:rm -rf /`), and `fail_closed_when_no_prompter` proves the non-TUI failure mode.
- Workspace tests: **1566 passed / 0 failed.**

**Deviation from spec:** task asked for "TUI prompt UI" wiring. The TUI prompt already exists (`CliModalPrompt` via `permissions/prompt.rs`) and shows the three-tier modal — what changed in T-0276 is the *granularity* of the grant the prompt produces, not the prompt UI itself. No new TUI surface was needed.

**Note on resume:** session got interrupted mid-task by a TCC / sandbox lockout on `/Users/dstorey/Desktop/arawn`. All files were intact on resume; only the final workspace test sweep + commit had not run. Both have now completed.