---
id: gui-f1-web-gui-foundation-serve
level: task
title: "GUI-F1: Web GUI foundation — serve route + asset embedding from the arawn binary"
short_code: "ARAWN-T-0491"
created_at: 2026-06-13T16:02:45.186670+00:00
updated_at: 2026-06-13T16:02:45.186670+00:00
parent: ARAWN-I-0070
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0070
---

# GUI-F1: Web GUI foundation — serve route + asset embedding from the arawn binary

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0070]]

## Objective **[REQUIRED]**

Stand up the web-GUI serving foundation: the `arawn` binary serves a build-time-embedded asset bundle + a GUI HTTP route, in the same process as the existing WS-RPC server, with the single-binary / ARM64 story preserved. This is the floor every other GUI task builds on.

### Type
- [x] Feature — GUI foundation

### Priority
- [x] P1 - High

## Acceptance Criteria **[REQUIRED]**

- [ ] Static assets are embedded at build time (pick `rust-embed` vs `include_dir` and justify) and served by the `arawn` binary under a GUI route (e.g. `/` or `/ui`).
- [ ] Serving shares the process/listener with `ws_server`; no new runtime dependency; `cargo build --release` is still a single binary and ARM64-buildable.
- [ ] A placeholder index page loads in a real browser against a running `arawn serve` (end-to-end serve path proven).
- [ ] Asset dev-iteration story documented (embedded in release; filesystem/live-reload in dev) OR live-reload explicitly deferred with a note.
- [ ] Test: the asset route returns 200 + correct content-type for an embedded asset and 404 for a missing one. `angreal check all` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
The HTTP layer is axum (same stack as `ws_server`). Add a static-file handler backed by the embedded dir and mount it alongside the WS upgrade route so both share the bind. Choose the embed crate in design; keep the bundle small.

### Dependencies
First foundation task — no task deps. Carries the initiative's embedding-mechanism decision.

### Risk Considerations
Don't bloat the release binary or break the existing WS upgrade route. Keep the GUI route behind the same bind/host the WS server already validates (ties into GUI-G2).

## Status Updates **[REQUIRED]**

*To be added during implementation*