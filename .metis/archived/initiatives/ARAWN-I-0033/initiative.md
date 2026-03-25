---
id: test-coverage-campaign
level: initiative
title: "Test Coverage Campaign"
short_code: "ARAWN-I-0033"
created_at: 2026-03-22T00:39:13.585195+00:00
updated_at: 2026-03-22T23:48:51.357894+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: test-coverage-campaign
---

# Test Coverage Campaign Initiative

## Context

Testing analysis (March 2026) found 3,196 tests across 20 crates with 68.7% measured coverage — but only 12 of 20 crates have coverage measurement. The 6 largest crates (arawn-agent, arawn-server, arawn-llm, arawn-oauth, arawn-plugin, arawn-workstream) have NO coverage measurement. Critical paths like WebSocket lifecycle, shell tool security, and session persistence have zero or shallow tests.

## Goals & Non-Goals

**Goals:**
- Achieve coverage measurement across ALL 20 crates
- Add tests for all critical security paths (shell tool, sandbox, auth)
- Add WebSocket lifecycle tests (currently zero)
- Add session persistence tests (currently zero)
- Wire up dead test code in gate.rs and registry.rs
- Add CI coverage thresholds to prevent regression
- Expand property-based testing beyond arawn-workstream

**Non-Goals:**
- 100% line coverage (diminishing returns)
- GUI/TUI visual testing
- Load testing or benchmarks (separate effort)

## Detailed Design

### Coverage Infrastructure (P0)
1. **Add coverage measurement for remaining 8 crates** in CI: arawn-agent, arawn-server, arawn-llm, arawn-oauth, arawn-plugin, arawn-workstream, arawn-tui, arawn-sandbox
2. **Add CI coverage thresholds**: Fail build if coverage drops below baseline for any crate
3. **Parallelize CI test execution**: Currently `--test-threads=1` serializes everything. Profile which tests actually need serialization vs can run parallel.

### Critical Security Tests (P0)
4. **Shell tool security tests** (`arawn-agent/src/tools/shell.rs`): Test blocklist bypass attempts, metacharacter injection, PTY mode, output truncation, timeout enforcement
5. **Sandbox escape tests** (`arawn-sandbox/src/`): Test blocked path access, denied syscalls, resource limits
6. **Auth tests**: Token validation edge cases, expired tokens, malformed headers
7. **Wire up dead gate.rs test code** (`arawn-agent/src/tool/gate.rs:622-646`): Security-critical test scaffolding exists but was never connected

### WebSocket & Server Tests (P1)
8. **WebSocket connection lifecycle**: Auth handshake, session creation, message exchange, idle timeout, reconnection, error propagation
9. **WebSocket concurrent connections**: Multiple clients, session ownership conflicts
10. **Rate limiting integration tests**: Verify per-IP limiting, burst handling, recovery

### Persistence & Data Tests (P1)
11. **Session persistence tests** (`arawn-session/src/persistence.rs`): Save/load/eviction/corruption recovery
12. **Memory store concurrency tests**: Concurrent reads/writes, graph operations under load
13. **Workstream message store tests**: JSONL append/read/corruption recovery

### CLI & Integration Tests (P2)
14. **CLI command execution tests**: All 16 commands with various flag combinations
15. **End-to-end test**: Start server → `arawn ask "hello"` → verify streaming response
16. **Property-based testing expansion**: Add proptest to arawn-config (path resolution), arawn-memory (store operations), arawn-mcp (protocol parsing)

## Implementation Plan

- Phase 1: Coverage infrastructure + critical security tests (1 week)
- Phase 2: WebSocket + persistence tests (1 week)
- Phase 3: CLI + integration + property-based tests (1 week)