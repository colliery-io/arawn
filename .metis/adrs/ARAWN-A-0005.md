---
id: 001-web-gui-as-primary-review-surface
level: adr
title: "Web GUI as primary review surface; TUI demoted to chat client"
number: 1
short_code: "ARAWN-A-0005"
created_at: 2026-06-11T10:59:17.285780+00:00
updated_at: 2026-06-11T11:06:42.159873+00:00
decision_date: 
decision_maker: 
parent: 
archived: false

tags:
  - "#adr"
  - "#phase/decided"


exit_criteria_met: false
initiative_id: NULL
---

# ADR-1: Web GUI as primary review surface; TUI demoted to chat client

## Context **[REQUIRED]**

Arawn is two products sharing a brain: (1) **interactive chat** — a linear, streaming, text conversation with the agent — and (2) **the ambient brain** — daily briefs (ceremony tablets), action items to review/snooze/dismiss, signals across lenses, feed health, memory inspection, and extraction provenance. The vision names the second half as the point ("watches, checks, summarizes, and nudges").

The 2026-06-11 full-codebase review (ARAWN-I-0067) found the TUI is adequate for chat but structurally fighting its medium for the review/triage half: width-aware table wrapping bugs (I-0067 P3-2), session identity crammed into an 8-char status-bar ID (P3-1), permission modals with no room for context (P3-3), briefs rendered as markdown in a chat pane, ceremony output reachable only by polling, and no viable surface for inbox-style triage (scan 20 items, dismiss 15, snooze 3, open 2). Each of these is a grind in ratatui and trivial in HTML. Continuing to invest TUI experience work means paying a permanent medium-mismatch tax on the product's differentiating half.

The architecture already anticipated this: the engine speaks `EngineRequest`/`EngineEvent` over a WebSocket server, the TUI is already just a client, and the `ArawnService` trait (~40 methods) defines the full UI↔backend contract. The vision's "Future Directions" section explicitly describes the TUI becoming "one client among many." A GUI is therefore an additive client, not a pivot.

The vision's scope constraint "TUI as primary interface (no web UI in v1)" was written to prevent UI scope creep before foundations existed. The foundations now exist; the constraint is being amended deliberately at the vision level, not drifted past.

## Decision **[REQUIRED]**

1. **A web GUI becomes the primary surface for review, triage, and observability** — briefs, action items, signals, feed/ceremony/steward health, memory inspection, extraction provenance.
2. **The web UI is served by the `arawn` binary itself** — static assets embedded at build time (e.g. `rust-embed`), talking to the same WebSocket/RPC server the TUI uses. No Node.js or other runtime dependency ships in the binary; frontend tooling runs at build time only. The single-binary, small-footprint, ARM64-deployable story is preserved.
3. **The TUI is demoted to a chat client, not removed.** `arawn tui` remains the fast terminal path for conversation and stays maintained for correctness, but review/triage/observability UX is no longer built or polished there.
4. **All review/observability capabilities are protocol-first**: every surface (status, ceremonies, action items, signals, memory) is an RPC on `ArawnService` rendered thinly by clients. No client-side-only features.
5. **Sequencing discipline:** the GUI initiative does not start until I-0067 Phases 1–2 (chat correctness + background-brain trustworthiness) are landed. Backend trust first; the GUI gets to be thin.

## Alternatives Analysis **[CONDITIONAL: Complex Decision]**

| Option | Pros | Cons | Risk Level | Implementation Cost |
|--------|------|------|------------|-------------------|
| Keep TUI as primary; invest Phase-3 polish | No new codebase; one interaction model | Permanent medium-mismatch tax on review/triage; tables, modals, dashboards all fight ratatui; unreachable from other devices | Medium (product stagnates on its differentiator) | Low per item, unbounded over time |
| Web UI served from the binary **(chosen)** | Fits self-hosted daemon model; reachable from phone/couch; single binary preserved; HTML native for inbox/dashboard UX; vision already anticipated multi-client | Second frontend codebase with its own build pipeline and gravity; WS/auth surface needs hardening beyond localhost | Low-Medium | Medium (own initiative) |
| Tauri desktop app | Rust-native backend affinity; system webview, small footprint | Ties UI to one machine with a display — backwards for a daemon headed to an ARM box; can be added later as a shell around the web UI, reverse migration doesn't exist | Medium | Medium-High |
| Pure-Rust native GUI (egui/iced/Dioxus-native) | No JS toolchain at all | Text-heavy, markdown-heavy, form-heavy apps are exactly these toolkits' weak spot; trades ratatui's rough edges for different rough edges | Medium-High | High |
| Electron | Mature ecosystem | Violates vision constraints outright (heavy runtime, Node.js) | — | Rejected on constraints |

## Rationale **[REQUIRED]**

- **The medium should match the shape of the data.** Chat is a stream; the TUI keeps it. Briefs, action items, signals, and health are inboxes/dashboards; HTML renders those natively where ratatui requires hand-built wrapping, scrolling, and modal machinery (the exact P3 findings of I-0067).
- **A self-hosted personal assistant's review surface should be reachable from anywhere you are** — including a phone. Only a served web UI satisfies that; every desktop option anchors the UI to one machine while the daemon runs on another.
- **The marginal architectural cost is low by prior design.** The WS server, channel protocol, and `ArawnService` trait already exist; the GUI is another consumer. Choosing anything that bypasses that contract (native toolkit with in-process coupling) would erode the one boundary that's been kept clean.
- **Build-time-only frontend tooling keeps the constraint that matters** (no heavy *runtime*, single binary, ARM64) while conceding the constraint that doesn't (no JS anywhere ever).

## Consequences **[REQUIRED]**

### Positive
- Review/triage UX moves to a medium that fits it; the differentiating half of the product stops paying the ratatui tax.
- I-0067 Phase 3 shrinks to cheap correctness fixes — experience work is not built twice.
- Protocol-first discipline (decision #4) keeps `ArawnService` the single contract; a future mobile/desktop shell is a rendering exercise.
- Briefs and action items become reachable from any device on the network.

### Negative
- A second frontend codebase with its own build pipeline, framework choice, dependency churn, and design upkeep.
- The WS server's auth/binding story (token file, 0.0.0.0 warnings — see I-0067 P1-4) must be hardened before the UI is exposed beyond localhost.
- Two clients to keep protocol-compatible during migration; TUI risks bit-rot if "demoted" drifts into "abandoned" — chat correctness in the TUI remains a maintained commitment.

### Neutral
- Vision scope constraint "TUI as primary interface (no web UI in v1)" must be amended to "TUI is the chat client; the served web UI is the review surface" (pending edit to ARAWN-V-0001).
- The GUI itself is a separate initiative (to be scaffolded when I-0067 Phases 1–2 near completion); frontend stack selection (Leptos / embedded SPA build / server-rendered htmx-style) is deliberately deferred to that initiative's discovery phase.
- I-0067 Phase 3 restructured accordingly (see that document's revision history).

## Review Schedule **[CONDITIONAL: Temporary Decision]**

### Review Triggers
- I-0067 Phases 1–2 complete and the GUI initiative is being scoped (stack selection happens then).
- Evidence the web UI cannot deliver a needed interaction (e.g., latency-sensitive streaming chat ergonomics) — would expand rather than reverse this decision.
- The single-binary constraint becomes violated in practice (asset embedding bloat, build-pipeline fragility).