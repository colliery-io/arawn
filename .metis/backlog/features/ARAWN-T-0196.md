---
id: sandbox-permissions-user-facing
level: task
title: "Sandbox/permissions: user-facing docs and audit view"
short_code: "ARAWN-T-0196"
created_at: 2026-05-02T00:00:00+00:00
updated_at: 2026-05-02T00:00:00+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


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

Code work still open under this ticket:
- [ ] Sandbox-denial errors carry user-facing messages identifying which restriction tripped
- [ ] `/permissions` TUI command — active rule set + recent decisions
- [ ] Engine error chain (T-0191) carries `denied by permission rule: <rule>` instead of generic "tool failed"

The docs page already mentions the audit-view as TODO and points users at `RUST_LOG=arawn=debug` for now.
