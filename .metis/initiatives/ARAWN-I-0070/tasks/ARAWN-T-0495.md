---
id: gui-s1-brief-ceremony-tablet
level: task
title: "GUI-S1: Brief / ceremony-tablet surface"
short_code: "ARAWN-T-0495"
created_at: 2026-06-13T16:02:50.755483+00:00
updated_at: 2026-06-13T17:52:33.272087+00:00
parent: ARAWN-I-0070
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0070
---

# GUI-S1: Brief / ceremony-tablet surface

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0070]]

## Objective **[REQUIRED]**

Render the daily/weekly **brief and ceremony tablets** in the GUI, refreshing on the `briefing_ready` push — the ambient-brain centerpiece, in a medium that fits it (vs. markdown-in-a-chat-pane today).

### Type
- [x] Feature — GUI surface

### Priority
- [x] P2 - Medium

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] Brief/tablet view renders structured `ceremonies.get_by_period` / `list_items` / `list_priorities` / `get_diary` data.
- [ ] A `briefing_ready` push refreshes the view without a full reload.
- [ ] Priority confirm/reject + diary actions wired through the existing RPCs (`ceremonies.confirm_priority` / `reject_priority` / `upsert_diary`), OR v1 ships read-only with the write actions filed as a fast follow.
- [ ] Large tablets remain usable (scroll/sections).
- [ ] Test: renders a sample tablet; a `briefing_ready` notice triggers refresh. `angreal check all` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Consume the structured `ceremonies.*` reads; render via templating; subscribe the view to `briefing_ready` through the GUI-F2 push bridge.

### Dependencies
GUI-F2. Reads the existing ceremony RPCs + the `briefing_ready` notice (I-0035 Phase 4).

### Risk Considerations
Write actions (priority confirm/reject) mutate state — confirm RPC coverage; if any action lacks an RPC, file it rather than hacking client-side.

## Status Updates **[REQUIRED]**

**2026-06-13 — Implemented & complete (read view + live refresh; writes deferred).**
- New `GET /brief` (`gui::brief_page`) gathers the current **daily** (today → yesterday fallback for tz boundaries) and **weekly** (current ISO week) tablets via `ceremony_service()` and renders them: per-section heading + status/generated, **Priorities** (with confirmed/done badges), items **grouped by section** (prettified labels, done = struck-through), and the **Diary**. Reads `get_by_period` / `list_items` / `list_priorities` / `get_diary` — all the structured ceremony RPCs.
- Rendering is split into a pure `brief_markup(&[BriefSection])` (testable without a service) + `gather_brief`/`build_section` (the cer.* fetches). `item_text()` extracts text/summary/headline/… from the opaque item `body` JSON with a compact fallback.
- **Live refresh without a full reload:** `notice_fragment` now adds a `refresh: "/brief #brief-body"` hint to `briefing_ready` fragments; the embedded client (`app.js`) fetches `/brief`, extracts `#brief-body`, and swaps it in place when that container is on the current page. So a `briefing_ready` push updates the home `#brief-status` card *and* re-pulls the open `/brief` view in place. (Generic notices carry no refresh hint.)
- **Writes deferred (read-only v1):** priority confirm/reject + diary edits have RPCs (`confirm_priority`/`reject_priority`/`upsert_diary`) but the GUI ships read-only this slice; interactive write actions are a fast-follow (clean fit for the same fragment-swap pattern). Recorded here rather than dropped.
- **Tests (6 new, 19 total in `ws_server::gui`):** empty placeholder + active nav; full tablet (priorities/items/diary/badges) render; item grouping+ordering; `item_text` key precedence + fallback; `section_label` prettify; `briefing_ready` fragment carries the refresh hint (generic does not).
- **Verified end-to-end** (restarted `arawn serve` pid 97172): `GET /brief` → 200 `text/html` rendering a **real daily tablet** (the boot back-fill had composed one), `#brief-body` present, active nav; `/` and `/health` still 200.
- Gate clippy + fmt clean.

Next: GUI-S2 (action-item inbox — the differentiator).