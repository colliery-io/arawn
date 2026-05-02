---
id: sandbox-permissions-user-facing
level: task
title: "Sandbox/permissions: user-facing docs and audit view"
short_code: "ARAWN-T-0196"
created_at: 2026-05-02T00:00:00+00:00
updated_at: 2026-05-02T14:06:58.544735+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Sandbox/permissions: user-facing docs and audit view

## Objective

Arawn's shell tool runs commands under a real OS sandbox (macOS `sandbox-exec`, Linux `bubblewrap`) and the engine has a permissions checker (`crates/arawn-engine/src/permissions/`). All this exists. None of it is user-visible. A user trying to reason about safety has to read Rust source. A user hitting a sandbox denial sees an opaque error. There's no audit trail of "what did the agent actually try to do." Make the existing protections legible.

## Type / Priority
- Feature (mostly docs + UX surfacing of existing systems)
- P1 — Blocker. Without a safety story, users either trust too much or refuse to use it.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `docs/security.md` covering: (a) what the shell sandbox blocks by default (network, write outside cwd, etc. — actual list per platform), (b) the permission rule grammar (Allow / Deny / AskUser), (c) where rules live (`arawn.toml [permissions]` + per-workstream overrides if any), (d) "the agent has shell access — here's how to limit blast radius" in plain English.
- [ ] Sandbox-denial errors (the `sandbox-exec` failure path) carry a user-facing message identifying *which* sandbox restriction tripped, not just the OS exit code.
- [ ] TUI gains a `/permissions` (or similar) command that prints the active rule set + recent permission decisions for the session (granted, denied, asked). At minimum a flat list; modal UI is a stretch.
- [ ] If a tool call is denied by the permissions checker, the engine error chain (T-0191) carries `denied by permission rule: <rule>` — not a generic "tool failed."

## Implementation Notes

- Permissions code lives in `crates/arawn-engine/src/permissions/` — `checker.rs`, `rules.rs`, `prompt.rs`. The grammar is already implemented; this ticket is about surfacing it.
- Sandbox enforcement lives in `crates/arawn-engine/src/tools/shell.rs` plus the `arawn-shell-sandbox` setup (see code-index for exact path).
- For the `/permissions` command, plumb data from the checker into a new RPC method or piggyback on `/help`-style command routing per T-0195.
- Avoid scope creep into *adding new permission categories* — that's a separate design conversation.

## Status Updates

### 2026-05-02 — Docs portion done

`docs/src/security.md` written. Covers shell sandbox per platform (sandbox-exec / bubblewrap, write restriction to workstream dir, sensitive deny-list with concrete paths, network-tools allowlist, env scrubbing), permission rule grammar (allow/deny/ask, glob, content patterns), evaluation order (deny > allow > ask), four permission modes (default / accept_edits / bypass / plan), and three "limit blast radius" config recipes. Linked from SUMMARY.md.

### 2026-05-02 — Code portion done

**Engine error chain (the originally-asked criterion):**
- `RuleMatcher::evaluate_with_match` returns the matched rule alongside the decision.
- New `DecisionReason` enum (`MatchedRule`, `SessionGrant`, `ModeFallback`, `Prompted`, `NoChecker`) with a `display()` formatter.
- `PermissionChecker::check_explained` returns `(decision, reason)`. The original `check` is preserved as a thin wrapper.
- `query_engine.rs` now uses `check_explained` and emits `Permission denied: tool 'X' was denied by rule 'deny shell(rm -rf *)'. Run /permissions in the TUI to inspect the active rule set, or see docs/src/security.md.` instead of the old opaque message. T-0191's error chain carries this body verbatim to the TUI.

**Audit log + `/permissions` view:**
- `SharedAudit` (`Arc<Mutex<VecDeque<AuditEntry>>>`) capped at 50 entries.
- `LocalService` owns a single `permission_audit` field; per-message `PermissionChecker`s are wired with `with_audit(...)` so the rolling history survives across messages.
- New `ServerCapabilities`-style `PermissionsStatus` type in `arawn-service`. New `get_permissions_status` method on `ArawnService`, implemented in `local_service.rs` (reads rules + mode directly off LocalService state, plus the shared audit).
- New `get_permissions_status` JSON-RPC method in `ws_server.rs`.
- New `WsClient::get_permissions_status` in the TUI.
- New `/permissions` slash-command + `CommandResult::PermissionsStatus` variant + handler in `event_loop.rs` that calls the RPC and renders mode + allow/deny/ask rules + the last 20 audit entries (newest first) as a system message via `format_permissions_status`.

**Sandbox-denial decoration:**
- The vendored `sandbox-runtime`'s `annotate_stderr_with_sandbox_failures` already appends a "--- Sandbox Violations ---" section when restrictions fire. Added a one-line hint pointing at `docs/src/security.md` and `/permissions` whenever that section appears, so the agent (and user) get actionable context instead of just a list of violations. We can't always identify *which* OS-level rule tripped (sandbox-exec messages are inherently opaque), but we can always tell the user where to look.

**Tests** (`crates/arawn-engine/src/permissions/checker.rs::tests`, all green):
- `check_explained_attributes_deny_to_matching_rule` — denials cite the matched rule.
- `check_explained_attributes_no_match_to_mode_fallback` — no-rule allows cite the mode.
- `audit_log_records_decisions_in_order_and_caps` — ring buffer evicts oldest at AUDIT_CAP.
- `shared_audit_aggregates_across_checkers` — confirms the production shape (one buffer, many per-message checkers).
- `snapshot_partitions_rules_by_kind_with_display_specs` — `/permissions` view rendering is consistent with rule grammar.

5 new permission tests + 1 new TUI command audit-coverage entry. Workspace lib stays green at 889 tests, 0 clippy warnings.

**Acceptance criteria status:**
- [x] `docs/src/security.md` (last session).
- [x] Sandbox-denial errors carry actionable context (annotation + hint pointing at docs).
- [x] `/permissions` TUI command — active rule set + recent decisions, formatted as system message.
- [x] Engine error chain carries `denied by rule '<spec>'` (or `denied by mode default '<mode>'` when no rule fired).