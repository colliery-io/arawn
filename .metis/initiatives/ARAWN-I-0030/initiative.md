---
id: bug-fixes-crash-prevention
level: initiative
title: "Bug Fixes & Crash Prevention"
short_code: "ARAWN-I-0030"
created_at: 2026-03-22T00:39:10.349985+00:00
updated_at: 2026-03-22T14:06:26.566602+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: S
initiative_id: bug-fixes-crash-prevention
---

# Bug Fixes & Crash Prevention Initiative

## Context

Deep bug-hunting analysis of Arawn (March 2026) found 5 confirmed bugs, 5 likely bugs, and multiple race conditions/crash scenarios. Several are in hot code paths (agent turn loop, memory store, OAuth token refresh). These must be fixed before relying on Arawn for daily use.

## Goals & Non-Goals

**Goals:**
- Fix all confirmed bugs (unsafe Send+Sync, MCP OOM, expect() panics, TOCTOU races)
- Investigate and fix likely bugs (token refresh race, session cache double-load, reconnect token timing)
- Make atomic all file writes that could corrupt data on crash (secret store, OAuth tokens)
- Eliminate all `unwrap()`/`expect()` calls in production code paths

**Non-Goals:**
- Performance optimization (separate initiative)
- New feature work
- Refactoring beyond what's needed to fix the bugs

## Detailed Design

### Confirmed Bugs (P0)
1. **Unsafe Send+Sync on MemoryStore** (`arawn-memory/src/store/mod.rs:72-73`): Wrap `GraphStore` in `parking_lot::Mutex` or `RwLock`. The current unsafe impl relies on single-threaded access pattern that is not enforced.
2. **MCP Content-Length unbounded allocation** (`arawn-mcp/src/transport.rs:330`): Add sanity check — reject Content-Length > 64MB (configurable). Return error instead of allocating.
3. **WebFetchTool expect() panics** (`arawn-agent/src/tools/web.rs:65,76`): Replace `expect()` with proper error propagation via `?` operator.
4. **Agent turn loop unwrap() calls** (`arawn-agent/src/agent.rs:215,291,392`): Replace with proper error handling or `.ok_or_else()`.
5. **TOCTOU in FileWriteTool** (`arawn-agent/src/tools/file.rs:328-334`): Use `OpenOptions` with appropriate flags for atomic check-and-open.

### Likely Bugs (P1)
6. **Token refresh race** (`arawn-oauth/src/token_manager.rs:170-188`): Hold the write lock through the entire check-refresh-save sequence; add mutex for the refresh operation.
7. **Session cache double-load race** (`arawn-session/src/cache.rs:147-191`): Use a single lock acquisition for the check-load-insert sequence, or use `entry()` API.
8. **Reconnect token timing attack** (`arawn-server/src/state.rs:1174`): Replace `!=` with `ct_eq` from the `subtle` crate, matching the pattern in auth.rs.

### Crash Prevention (P1)
9. **Atomic secret store writes** (`arawn-config/src/secret_store.rs:129-159`): Write to temp file, then `rename()` for atomic swap.
10. **Atomic OAuth token writes** (`arawn-oauth/src/token_manager.rs:136`): Same write-to-temp-then-rename pattern.
11. **std::sync::Mutex in async context** (`arawn-memory/src/store/unified_ops.rs:39`): Evaluate migration to `tokio::sync::Mutex` or use `spawn_blocking` for SQLite operations.

### Error Handling Gaps (P2)
12. **Session cache update silently drops errors** (`arawn-server/src/state.rs:1009`): Log warning on failure.
13. **Eviction callback errors swallowed** (`arawn-session/src/cache.rs:156,210,334,350`): Log warning on failure.
14. **Secret store permissions failure ignored** (`arawn-config/src/secret_store.rs:155`): Log warning, consider failing if permissions can't be set.
15. **Plugin hook stdin write failure ignored** (`arawn-plugin/src/hooks.rs:581-582`): Log warning.
16. **WebSocket stream break loses work** (`arawn-server/src/routes/ws/connection.rs:178-185`): Signal stream producer to stop on send failure.

## Implementation Plan

- Phase 1: P0 confirmed bugs (1-2 days)
- Phase 2: P1 likely bugs + crash prevention (2-3 days)
- Phase 3: P2 error handling gaps (1 day)