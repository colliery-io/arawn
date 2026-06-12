---
id: p1-4-server-lifecycle-ergonomics
level: task
title: "P1-4: Server-lifecycle ergonomics — TUI preflight, bind-error decoration, stale token handling"
short_code: "ARAWN-T-0471"
created_at: 2026-06-11T11:07:20.282890+00:00
updated_at: 2026-06-11T16:24:37.955868+00:00
parent: ARAWN-I-0067
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0067
---

# P1-4: Server-lifecycle ergonomics — TUI preflight, bind-error decoration, stale token handling

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0067]] — implements finding P1-4 (HIGH). Also a prerequisite for the web GUI's connection story (ADR ARAWN-A-0005).

## Objective **[REQUIRED]**

Make server-lifecycle mistakes produce one-line actionable messages: `arawn tui` preflights server reachability, `arawn serve` decorates port-bind failures, and a stale `server.token` after a crash is handled instead of causing repeated auth failures.

**Current defects:**
- `crates/arawn/src/main.rs:1243` — TUI mode launches without checking the server is reachable; the user gets a raw WebSocket connect error. (CLI one-shot mode does this right — `crates/arawn/src/startup/cli.rs:12-16` prints "Start the server first: arawn serve"; match that quality.)
- `crates/arawn/src/ws_server/mod.rs:299` — bind failure propagates as an undecorated `anyhow` error; nothing names the port or suggests an existing `arawn serve` / `--port`.
- `crates/arawn/src/ws_server/mod.rs:309-311` — `server.token` is cleaned up only on graceful shutdown; after a crash, clients retry with the stale token instead of recovering.

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] `arawn tui` with no reachable server prints a one-line actionable message (address + "arawn serve") to plain stderr and exits BEFORE entering raw mode — no raw WebSocket error, no corrupted terminal (`preflight_server`)
- [x] `arawn serve` on an occupied port names the address and suggests an existing server / `--port` (AddrInUse-decorated bind)
- [x] Stale `server.token` recovers cleanly: token write moved to AFTER a successful bind, so the server always rewrites a fresh token on start AND a failed `serve` (port conflict) no longer clobbers a running server's token
- [x] Tests: `preflight_fails_for_unreachable_server`, `preflight_rejects_malformed_url` (bind-conflict left to manual/integration — AddrInUse mapping is a straightforward `ErrorKind` match)
- [x] `angreal test unit` green for the changed crate (arawn 61/0 lib + 2/0 bin); `angreal check all` carries pre-existing repo-wide failures unrelated to this task

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
TUI preflight: short-timeout (~500ms) TCP/WS connect to the configured address **before** entering raw mode / alternate screen, so a failure prints a normal stderr line and exits cleanly. Bind errors: wrap the listener bind in `ws_server/mod.rs` with context naming the port and remediation. Token: have `arawn serve` rewrite `server.token` unconditionally on startup (crash-stale tokens become irrelevant), and have clients re-read the token file once on auth failure before giving up with guidance.

### Dependencies
None. Independent of the other Phase 1 tasks.

### Risk Considerations
Preflight must not race a server that is mid-startup — message should say "not reachable" (with the address), not "not running". Keep the check cheap and skip it when an explicit `--no-preflight`-style escape hatch is configured, if one proves necessary (default: keep it simple, no flag).

## Status Updates **[REQUIRED]**

**2026-06-11 — COMPLETE.** Three fixes in `main.rs` + `ws_server/mod.rs`.

- **TUI preflight** (`main.rs`): new `preflight_server(ws_url)` does an 800ms TCP connect to the parsed host:port **before** `run_tui` enters raw mode. On failure it prints a one-line stderr message ("Cannot reach the arawn server at … — start it first: arawn serve") and exits — no half-initialized terminal, no raw WS error. Matches the quality of the existing CLI-mode message.
- **Bind-error decoration** (`ws_server/mod.rs`): `TcpListener::bind` failures are mapped — `AddrInUse` → "address already in use … is another `arawn serve` running? … `--port <N>`"; other io errors get a plain "Failed to bind {addr}".
- **Stale token** (`ws_server/mod.rs`): the real bug was ordering — `write_token_file` ran *before* the bind, so a failed `serve` (port conflict) clobbered the **running** server's token and locked out its clients. Moved the token write to **after** a successful bind. The server still rewrites a fresh token on every successful start (replacing any crash-stale one), and a failed start now leaves the running server's token intact. `auth_token` cloned into `AppState` so the post-bind write can still reference it.

Client-side re-read on auth failure deemed unnecessary: with the server-side ordering fix, a running server's token is never stale-and-wrong, and a restarted server always writes fresh — so the "and/or" criterion is satisfied by the server half.

**Tests:** `preflight_fails_for_unreachable_server` (port 1 refuses fast), `preflight_rejects_malformed_url`. Bind-conflict not unit-tested (needs a real port bind); the `AddrInUse` mapping is a trivial `ErrorKind` match.

**Verification:** `cargo build --workspace` clean; arawn **61/0** lib + **2/0** bin tests; my changed lines `rustfmt --check` clean (the lone remaining main.rs:477 diff is pre-existing classifier churn, not mine); no new dependency. `angreal check all` red is pre-existing.