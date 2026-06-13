---
id: web-gui-primary-review-triage
level: initiative
title: "Web GUI — primary review/triage surface served by the arawn binary"
short_code: "ARAWN-I-0070"
created_at: 2026-06-13T15:41:25.946171+00:00
updated_at: 2026-06-13T16:49:33.537277+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: L
initiative_id: web-gui-primary-review-triage
---

# Web GUI — primary review/triage surface served by the arawn binary

Implements [[ARAWN-A-0005]] (Web GUI as primary review surface; TUI demoted to chat client). Scaffolded 2026-06-13 when [[ARAWN-I-0069]] (Phase 3: TUI correctness + GUI prerequisites) completed — the ADR's sequencing gate ("the GUI initiative does not start until chat correctness + background-brain trustworthiness are landed") is now satisfied.

## Context **[REQUIRED]**

Arawn is two products sharing a brain: (1) **interactive chat** (a linear streaming text conversation) and (2) **the ambient brain** (daily briefs / ceremony tablets, action items to review/snooze/dismiss, signals across lenses, feed/ceremony/steward health, memory inspection, extraction provenance). The TUI is adequate for chat but structurally fights its medium for the review/triage half — inbox/dashboard UX is native in HTML and a grind in ratatui (the I-0067 P3 findings).

ADR A-0005 decided a **web GUI served by the `arawn` binary itself** (static assets embedded at build time, talking to the existing WebSocket/RPC server the TUI already uses — no Node.js/other runtime in the shipped binary; frontend tooling runs at build time only). The TUI is demoted to a maintained chat client, not removed. Everything is **protocol-first**: every surface is an `ArawnService` RPC rendered thinly.

The I-0069 P3-7 readiness audit ([[ARAWN-T-0490]]) confirmed, by exercising the WS-RPC surface against the 35-test integration suite, that the **data plane a thin web client needs is present and structured**: versioned `status`/`health`, sessions+promotion, structured ceremony/todo reads, typed errors (incl. `llm_kind`), permission control + interactive approval loop, streaming, and push notices. The remaining gaps are **edge/transport** concerns, captured as the seed backlog below.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- A web GUI, served by the `arawn` binary, that is the primary surface for **review, triage, and observability**: briefs/tablets, action-item inbox (scan/dismiss/snooze/open), signals across lenses, feed/ceremony/steward health, memory inspection, extraction provenance.
- **Single-binary preserved**: static assets embedded (e.g. `rust-embed`); frontend build-time only; ARM64-deployable; no heavy runtime.
- **Protocol-first**: net-new capabilities land as `ArawnService` RPCs, not client-only features. Harden the WS auth/binding/transport story so the surface is safe beyond localhost.
- Reachable from any device on the network (incl. phone).

**Non-Goals:**
- Replacing the TUI. `arawn tui` remains the maintained fast terminal **chat** path; this initiative does not rebuild chat in the browser as a priority.
- Multi-user/multi-tenant accounts (single self-hosted operator assumed unless discovery says otherwise).
- A desktop/native shell (Tauri etc.) — explicitly deferrable per the ADR; the served web UI comes first.

## Discovery — open questions to resolve before design **[REQUIRED]**

These are deliberately deferred to this initiative (per ADR A-0005) and must be answered in discovery → design before decomposition:

1. **Frontend stack** — server-rendered (htmx/datastar-style over the existing WS) vs. an embedded SPA (Leptos / Dioxus-web / a JS framework built at build time). Trade-off: build-pipeline weight vs. interactivity. The ADR keeps "no JS *runtime* in the binary" but allows build-time JS.
2. **Asset embedding mechanism** — `rust-embed` vs. `include_dir`; dev-mode live reload vs. embedded-only.
3. **Transport for the browser** — reuse the existing WS-RPC envelope directly, or add a thin HTTP/JSON facade for initial page loads + WS for streaming/push.
4. **Auth model** — see GUI-G1/G2 below; decide the credential + origin story before exposing beyond localhost.
5. **Scope of v1 surfaces** — which review/triage surfaces ship first (likely: brief/tablet view + action-item inbox + health dashboard).

## Resolved decisions (2026-06-13) **[REQUIRED]**

Discovery questions answered by the operator:

1. **Frontend stack → server-rendered hypermedia.** HTML rendered by the `arawn` binary + a tiny hypermedia library (datastar or htmx; final pick in design) driving updates over SSE/WS. Rationale: thinnest fit for "thin client rendering RPCs", minimal/no JS build pipeline, single-binary stays trivial, and it suits list/form/dashboard + streaming UX. Trade-off accepted: richer interactivity costs more than a full SPA.
2. **Auth/exposure → localhost-first + Origin/CORS now.** v1 binds loopback and adds WS `Origin` + CORS validation (GUI-G2) so a browser works safely on localhost. Remote-auth hardening (GUI-G1) and TLS (GUI-G5) are deferred until the UI exists — they stay in the backlog, not dropped.
3. **v1 surface scope → all four** review/triage surfaces: brief/ceremony-tablet view, action-item inbox, health/observability dashboard, and signals/memory/extraction-provenance. (Build order still sequences cheapest-first; see plan.)

## Architecture (resolved direction) **[REQUIRED]**

- The `arawn` binary serves **server-rendered HTML** (embedded templates/assets at build time) **and** the existing WS-RPC server (`ws_server`). The browser is another `ArawnService` consumer — same contract as the TUI.
- **Hypermedia over the wire:** initial page loads are server-rendered HTML; live updates (streaming tokens, `briefing_ready` and other `ServerNotice`s, tablet/inbox changes) push as HTML fragments or signals over SSE/WS. No client-side state store; the server is the source of truth.
- **Protocol-first:** any net-new review/observability capability is an `ArawnService` RPC rendered thinly; no browser-only logic. Existing structured reads (`status`/`health`, `ceremonies.*`, `todos.*`, typed errors with `llm_kind`) back the surfaces directly.
- **Transport detail (design phase):** decide between (a) reusing the WS-RPC envelope and rendering fragments client-side from a minimal hypermedia runtime, vs. (b) a small HTTP route set for page loads + SSE/WS for push. Server-rendered-hypermedia favors (b) for first paint.
- **Single-binary preserved:** assets embedded (`rust-embed`/`include_dir` — design pick); frontend tooling build-time only; ARM64 deployable.

## Seed backlog **[REQUIRED]**

Captured from the [[ARAWN-T-0490]] readiness audit. These become formal child tasks at the **decompose** phase (after discovery/design + human approval). **Edge/transport hardening (GUI-G*) should land before the UI is exposed beyond localhost.**

**Transport / security hardening (do early):**
- **GUI-G1 — Auth beyond the localhost token file (P1).** Today: a single `server.token` bearer file checked at WS upgrade + a non-loopback startup warning ("arawn has no auth layer"). Needs a non-file credential story (token is plaintext on disk), TLS termination guidance, and per-client identity if multi-user ever enters scope. Extends I-0067 P1-4.
- **GUI-G2 — Browser Origin/CORS validation (P1).** Cross-origin browser clients need CORS + WS `Origin` checking; neither exists. Required before any cookie/header auth to avoid CSRF-style exposure.
- **GUI-G5 — TLS / reverse-proxy deployment note (P3).** Server binds plain TCP; document a TLS-terminating reverse-proxy story (or native TLS).

**Protocol ergonomics:**
- **GUI-G3 — Machine-readable protocol schema (P2).** `RPC_METHODS` is a Rust const + per-handler parsing; publish an OpenRPC/JSON-Schema (versioned alongside `SYSTEM_STATUS_VERSION`) so the client can codegen types and drift is caught.
- **GUI-G4 — Documented streaming-event envelope (P2).** Engine-event/notice envelope shapes are stable + tested but only defined in Rust; document them as the streaming contract.

**Foundations + first surfaces (sequence after discovery):**
- GUI-F1 — Build pipeline + asset embedding + dev/live-reload (depends on stack decision).
- GUI-F2 — Serve route(s) from the `arawn` binary; wire WS auth to browser sessions.
- GUI-S1 — Brief / ceremony-tablet view (consumes structured `ceremonies.*` reads + `briefing_ready` push).
- GUI-S2 — Action-item inbox (scan / dismiss / snooze / open) — the differentiating triage surface.
- GUI-S3 — Health/observability dashboard (consumes versioned `status`/`health`).
- GUI-S4 — Signals across lenses + memory inspection + extraction provenance.

## Alternatives Considered **[REQUIRED]**

Settled at the ADR level (see [[ARAWN-A-0005]] alternatives table): keep TUI-primary (rejected — permanent medium-mismatch tax), Tauri desktop (deferred — anchors UI to one machine; the daemon is headed for an ARM box), pure-Rust native GUI (rejected — text/form-heavy apps are these toolkits' weak spot), Electron (rejected on the no-heavy-runtime constraint). **Chosen: web UI served from the binary.** Open sub-alternatives (frontend stack, transport facade) are the discovery questions above.

## Implementation Plan **[REQUIRED]**

Phased, human-in-the-loop at each transition:
1. **Discovery** (current) — answer the open questions; pick the frontend stack + transport + auth model.
2. **Design** — concrete architecture: serve route, asset embedding, auth/CORS design, protocol-schema approach, v1 surface set.
3. **Decompose** — turn the seed backlog into ordered tasks (GUI-G1/G2 first, then foundations, then surfaces).
4. **Active** — build hardening → foundations → brief/inbox/health surfaces, protocol-first throughout.

Sequencing rule (from the ADR): backend trust first (done — I-0067 P1–2, I-0068, I-0069), so the GUI gets to be thin.