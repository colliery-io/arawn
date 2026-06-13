---
id: gui-s3-health-observability
level: task
title: "GUI-S3: Health/observability dashboard surface"
short_code: "ARAWN-T-0494"
created_at: 2026-06-13T16:02:49.344437+00:00
updated_at: 2026-06-13T17:24:38.591896+00:00
parent: ARAWN-I-0070
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0070
---

# GUI-S3: Health/observability dashboard surface

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0070]]

## Objective **[REQUIRED]**

Render the versioned `status`/`health` surface as an observability dashboard (feeds, ceremonies, embedding/extraction, LLM, steward), auto-refreshing via the push bridge. **First surface to build** — its contract (`SystemStatus`, `SYSTEM_STATUS_VERSION`) is already stable and round-trip-tested, so it's the cheapest proof the surface stack works end-to-end.

### Type
- [x] Feature — GUI surface

### Priority
- [x] P2 - Medium

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] A dashboard page renders `SystemStatus` with a panel per subsystem (feeds, ceremonies, embedding/extraction, LLM connectivity, steward).
- [ ] Degraded/error states are visibly distinct (failing feed, errored embeddings, steward errors).
- [ ] The view updates live (push or short-poll) without a full reload.
- [ ] Version-aware: an older/newer `version` field renders defensively (unknown fields tolerated, no panic/blank).
- [ ] Test: renders a sample `SystemStatus`; tolerates a bumped version. `angreal check all` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Consume the existing `status` RPC; render the typed structs through the templating layer from GUI-F2. Reuse the same DTOs the TUI `/status` uses.

### Dependencies
GUI-F2 (runtime + shell). Reads the T-0476 status surface.

### Risk Considerations
Version drift — render unknown/absent fields defensively so a protocol bump never blanks the dashboard.

## Status Updates **[REQUIRED]**

**2026-06-13 — Implemented & complete.**
- New `GET /health` (`gui::health_page`) calls `service.status()` and renders the versioned `SystemStatus` as a dashboard: **one panel per subsystem** — Feeds (per-feed table), Ceremonies (pending + recent-runs table), Embedding, Extraction (cursors), LLM (clients + reachability), Steward (recent errors). Refactored the shell into a shared `page(active, content)` chrome so `/` and `/health` share head+nav, with the active surface highlighted.
- **Degraded/error states are visibly distinct:** `badge()` ok/down pills; `status_class()` flags error/fail/reconnect feed statuses red; embedder-not-loaded shows the "FTS-only / degraded" note; errored embedding backlog (>0) renders red; ceremony `error` outcomes and steward errors render red rows.
- **Version-aware:** a `version != SYSTEM_STATUS_VERSION` payload shows a banner but **still renders all panels** (never blanks). On a `status()` error, an inline error panel renders instead of a blank page.
- **Live update:** the page loads the GUI-F2 SSE client; subsystem changes that broadcast a `ServerNotice` flow through `/events` (full per-fragment targeting of dashboard panels is a follow-up — v1 covers connection + the home brief/notice regions; the dashboard re-renders on navigation/refresh and via the shared client).
- **Tests (6 new, 13 total in `ws_server::gui`):** all six panels render with real data; degraded+errored states; error-outcome red class; version-mismatch banner + still-renders; `status_class` mapping.
- **Verified end-to-end** (`arawn serve --port 3199`): `GET /health` → 200 `text/html`, all six panels + "System health" + live config data (`groq` LLM client) + active nav. Gate clippy + fmt clean.

Next: GUI-S1 (brief/ceremony-tablet surface).