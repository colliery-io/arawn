---
id: gui-g2-browser-safe-ws-origin-cors
level: task
title: "GUI-G2: Browser-safe WS — Origin + CORS validation (localhost-first)"
short_code: "ARAWN-T-0492"
created_at: 2026-06-13T16:02:46.502125+00:00
updated_at: 2026-06-13T17:03:52.809382+00:00
parent: ARAWN-I-0070
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0070
---

# GUI-G2: Browser-safe WS — Origin + CORS validation (localhost-first)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0070]]

## Objective **[REQUIRED]**

Make the WS/HTTP surface safe for a **browser** client on localhost: validate the WS `Origin` header and add CORS, so a page served by `arawn` can connect while cross-site origins are rejected. This is the v1 auth posture (localhost-first); remote auth (GUI-G1) and TLS (GUI-G5) stay deferred.

### Type
- [x] Feature — transport hardening (security)

### Priority
- [x] P1 - High

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] WS upgrade validates `Origin` against an allowlist (loopback + configured host:port); a disallowed *present* origin is rejected with a clear handshake/close error.
- [ ] HTTP/GUI routes set CORS headers for the same allowlist; preflight handled.
- [ ] The existing `server.token` bearer check still applies — the Origin check is additive, not a replacement.
- [ ] A non-browser WS client (the TUI) that sends **no** `Origin` is still accepted — only a present-and-disallowed origin is rejected.
- [ ] Config knob for allowed origins (default loopback), documented. Tests: allowed origin upgrades; disallowed origin rejected; missing-token still rejected; no-Origin (TUI) accepted.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
In the `ws_server` upgrade handler, read the `Origin` header and compare against an allowlist derived from the bind host:port + config. Add CORS via `tower-http`'s `CorsLayer` (or manual headers) for the HTTP routes.

### Dependencies
GUI-F1 (a served page to actually have an origin). Builds on the existing token-file auth (I-0067 P1-4).

### Risk Considerations
Must not lock out the TUI: the discriminator is *present-and-disallowed* origin, not *missing* origin. Keep the allowlist default tight (loopback only).

## Status Updates **[REQUIRED]**

**2026-06-13 — Implemented & complete.**
- New `ws_server/origin.rs`: `build_allowlist(host, port, extra)` (loopback defaults — `http://localhost|127.0.0.1|[::1]:<port>` — plus a concrete non-loopback bind host and operator extras; wildcard binds add no wildcard origin) and `check_origin(headers, allowlist) -> {Absent, Allowed, Denied}`.
- `ws_handler` now runs the Origin check **before** the token check: a *present-and-disallowed* `Origin` → `403 Forbidden`; a *missing* `Origin` (non-browser/TUI) → `Absent` → falls through to the existing token check. Origin check is additive — token auth unchanged.
- CORS via `tower-http` `CorsLayer` (`AllowOrigin::list(allowlist)`, GET/POST, preflight handled) layered on the router for the browser-facing routes.
- Config: `ServerConfig.allowed_origins: Vec<String>` (serde default empty — localhost-first), documented inline; threaded through `run_server(.., allowed_origins)` and the `main.rs` call site.
- **Tests:** 7 unit tests in `ws_server::origin` (loopback allowlist, non-loopback+extras, wildcard→no-wildcard, absent=allowed, present-allowed, present-disallowed incl. wrong-port, case-insensitive).
- **Verified end-to-end** (`arawn serve --port 3199`, real WS upgrade requests via curl): bad Origin → **403**; allowed Origin (no token) → **401** (origin passed, token reached); **no Origin (TUI) → 401** (not locked out, reaches token check); GUI route with allowed Origin → `access-control-allow-origin: http://localhost:3199`; CORS preflight `OPTIONS` → **200**.
- Regression: all 17 `arawn-tests/websocket.rs` integration tests still green (they connect via tungstenite with no Origin — confirms the TUI path is unaffected).
- `cargo test -p arawn --lib ws_server::origin` green; gate clippy + fmt clean.

**Note:** the websocket.rs harness builds its own minimal router (bypasses `ws_handler`), so the Origin check is covered by the unit tests + the curl end-to-end, not that harness. Next: GUI-F2 (hypermedia runtime + push bridge + shell).