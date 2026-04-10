---
id: security-hardening-path-traversal
level: initiative
title: "Security Hardening: Path Traversal, Auth, Sandbox, and Permission Bypass"
short_code: "ARAWN-I-0022"
created_at: 2026-04-09T23:59:35.045195+00:00
updated_at: 2026-04-10T01:08:41.573019+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: M
initiative_id: security-hardening-path-traversal
---

# Security Hardening: Path Traversal, Auth, Sandbox, and Permission Bypass Initiative

## Context

Architecture review identified a permission bypass chain (SEC-001 + SEC-002 + SEC-007) as the most exploitable security issue: grep/glob have no path restrictions, the WebSocket has no authentication, and `set_permission_mode` can remotely switch to bypass mode. A malicious webpage could open a WebSocket, set bypass mode, and instruct the LLM to search for credentials anywhere on disk.

**Review findings addressed:** R-02, R-03, R-09, R-10

## Goals & Non-Goals

**Goals:**
- Add path restrictions to grep and glob tools matching file_read's canonicalize+starts_with pattern (SEC-001)
- Implement sandbox for background shell commands or add mandatory permission gate (SEC-004)
- Add WebSocket authentication via session token (SEC-002, SEC-007)
- Surface sandbox failures as user-visible warnings instead of silent fallback (SEC-006)
- Display hook/MCP server commands during plugin installation (SEC-003, SEC-008)

**Non-Goals:**
- Full threat modeling or penetration testing
- Encrypted-at-rest session storage
- Network-level security (TLS, etc.) — local-only system

## Detailed Design

### Path Restrictions (R-02)
Extract the path validation logic from `file_read.rs` into a shared `validate_path(path, ctx)` function. Apply in `grep.rs` and `glob.rs`. Respect `ctx.allowed_paths` for escape hatches.

### Background Sandbox (R-03)
If OS sandbox can't support background processes, add a mandatory `Ask` permission check for `run_in_background: true` that cannot be overridden by session grants or bypass mode.

### WebSocket Auth (R-09)
Generate random token at startup → write to `~/.arawn/server.token` → TUI reads to authenticate → validate on WebSocket upgrade. Also validate `Origin` header to reject browser connections. Require modal confirmation for `set_permission_mode(bypass)`.

### Sandbox Failure Surfacing (R-10)
When sandbox is unavailable, emit `EngineEvent::Warning` (from ARAWN-I-0021). Log plugin hook registrations at info level. Add `plugin inspect` command.

## Implementation Plan

1. Path restrictions (Hours) — no dependencies
2. Session grant fix (Hours) — done in ARAWN-I-0021, prerequisite for sandbox work
3. Background sandbox gate (Days) — benefits from session grant fix
4. WebSocket authentication (Days) — independent
5. Sandbox failure surfacing (Days) — depends on EngineEvent::Warning from ARAWN-I-0021