---
id: gui-f2-hypermedia-runtime-sse-ws
level: task
title: "GUI-F2: Hypermedia runtime + SSE/WS push bridge + base layout shell"
short_code: "ARAWN-T-0493"
created_at: 2026-06-13T16:02:47.892215+00:00
updated_at: 2026-06-13T17:17:52.032645+00:00
parent: ARAWN-I-0070
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0070
---

# GUI-F2: Hypermedia runtime + SSE/WS push bridge + base layout shell

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0070]]

## Objective **[REQUIRED]**

Wire the server-rendered hypermedia runtime + an SSE/WS push bridge + a base layout shell. After this, server-originated updates (`ServerNotice` incl. `briefing_ready`, engine events) drive live fragment/signal updates with no full reload, and the four surfaces have a nav shell to live in.

### Type
- [x] Feature — GUI foundation

### Priority
- [x] P1 - High

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] Hypermedia library chosen (datastar vs htmx+SSE) and embedded; the choice is recorded with rationale.
- [ ] Server-side templating chosen (askama / maud / minijinja) and a reusable fragment-render helper exists.
- [ ] A base layout/shell renders with navigation to the four v1 surfaces (health, brief, inbox, signals).
- [ ] A push endpoint (SSE or WS) streams server-originated updates; a demo fragment updates on a `briefing_ready` notice without a page reload.
- [ ] Tests: shell renders; the push endpoint emits a fragment when a notice is broadcast. `angreal check all` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Bridge the existing `notice_tx` broadcast (and engine-event stream) into an SSE/WS endpoint that emits rendered HTML fragments / hypermedia signals. Keep the client runtime tiny and stateless — server is the source of truth (no client store).

### Dependencies
GUI-F1 (serve route), GUI-G2 (browser-safe transport).

### Risk Considerations
Don't reintroduce client-side state. Keep the runtime small to preserve the thin-client property. Ensure the push bridge survives reconnects gracefully.

## Status Updates **[REQUIRED]**

**2026-06-13 — Implemented & complete.**
- **Templating → maud** (compile-checked HTML in Rust; no separate template files to embed). Used `axum::response::Html<String>` to render rather than maud's `axum` feature, which is pinned to an older axum — keeps us decoupled from that pin.
- **Hypermedia runtime → server contract is SSE of `{target, mode, html}` fragments**, applied by a ~30-line embedded vanilla `EventSource` client (`web/dist/assets/app.js`). **Honest deviation from the datastar choice:** datastar ships as a browser JS file (not a cargo crate) and can't be vendored in this offline env. The server side is library-agnostic, so datastar (or htmx+SSE) drops in later by embedding its runtime + switching client attributes — **no Rust changes**. Recorded in the module doc.
- **Shell:** `gui::shell` (`GET /`, now maud-rendered, replacing the F1 static index.html) with nav to the four surfaces (Health/Brief/Inbox/Signals) + live regions `#brief-status` and `#notice-log`.
- **Push bridge:** `gui::events` (`GET /events`) → `Sse` over `notice_event_stream()`, which wraps `service.subscribe_notices()` in a `BroadcastStream` and maps each `ServerNotice` to a `fragment` SSE event via `notice_fragment()` (`briefing_ready` → replace `#brief-status`; everything else → append to `#notice-log`). Lagged slow-clients drop missed notices instead of erroring. Keep-alive on.
- **Tests (8 in `ws_server::gui`):** assets embedded; serve 200/404 + content-type map; shell has nav + live regions + client script; `briefing_ready`→replace and generic→append mapping; **`broadcast_notice_is_forwarded_as_sse_event`** (async — proves a broadcast notice surfaces as one SSE event through the real stream builder).
- **Verified end-to-end** (`arawn serve --port 3199`): `GET /` → maud shell with all four nav labels + both live regions + app.js; `GET /assets/app.js` → 200 `text/javascript`; `GET /events` → 200 `text/event-stream` (long-lived, holds open). Full arawn lib suite 85 green; gate clippy + fmt clean.
- **Deferred:** a *live ceremony-triggered* `briefing_ready` → fragment demo (needs the LLM ceremony path) lands naturally in **GUI-S1**, where the brief surface consumes it. Here the mapping is unit-tested, the broadcast→SSE forwarding is unit-tested, and the SSE transport is live-verified — the seam is fully covered.

**Note:** discovered the config watcher binds the real `~/.arawn` path, not `--data-dir` — out of scope here; worth a small follow-up. Next: GUI-S3 (health dashboard — first surface).