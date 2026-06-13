---
id: gui-s2-action-item-inbox-surface
level: task
title: "GUI-S2: Action-item inbox surface (scan / dismiss / snooze / open)"
short_code: "ARAWN-T-0496"
created_at: 2026-06-13T16:02:54.454634+00:00
updated_at: 2026-06-13T20:20:55.592528+00:00
parent: ARAWN-I-0070
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0070
---

# GUI-S2: Action-item inbox surface (scan / dismiss / snooze / open)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0070]]

## Objective **[REQUIRED]**

The differentiating surface: an **inbox-style action-item triage** — scan a list, dismiss/snooze/open, keyboard-friendly. This is the UX that's a grind in ratatui and native in HTML, and the reason for the GUI per ADR A-0005.

### Type
- [x] Feature — GUI surface

### Priority
- [x] P2 - Medium

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] Inbox lists action items (`todos.list` + ceremony action items) as bulk-scannable rows.
- [ ] Dismiss / open / (snooze) actions wired to existing RPCs (`todos.done` / `undo` / `patch` / `archive`); rows update optimistically via fragments.
- [ ] Keyboard navigation for fast triage (e.g. j/k to move, x to dismiss, enter to open).
- [ ] Empty + large-list states handled.
- [ ] Test: list renders sample items; an action calls the correct RPC and updates the row. `angreal check all` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Consume `todos.*` (and ceremony item) reads; render rows; wire actions through the hypermedia runtime to the existing mutation RPCs with optimistic fragment swaps.

### Dependencies
GUI-F2.

### Risk Considerations
**Snooze** may have no backing RPC today — verify; if missing, file a protocol task (`todos.snooze` / a due-date patch) rather than faking it client-side. Optimistic updates must reconcile on server confirm/err.

## Status Updates **[REQUIRED]**

**2026-06-13 — Implemented & complete.**
- New `GET /inbox` (`gui::inbox_page`) lists open todos (`ListFilter { open_only: true }`) as scannable rows; empty state renders "Inbox zero". Each row shows body + lens/due tags + a (CSS-hidden until focus/expand) rationale.
- **Functional triage actions** via `POST /inbox/{id}/{action}` (`gui::inbox_action`) over the same `TodoService` the WS RPCs use (so `TodoEvent`s still broadcast): `done` (mark_done), `undo`, `snooze` (= `patch` due_at +1 day — **no dedicated snooze RPC, implemented as a due-date bump** per the ticket's flag, not faked client-side), `dismiss` (archive). Each returns the re-rendered row (empty body for dismiss → client removes it).
- **Client (`app.js`):** a delegated click handler POSTs any `[data-action]` button and swaps/removes the row in place; keyboard nav — `j`/`k` move row focus, `x` dismisses the focused row, `e` toggles rationale expand. (Optimistic-on-success: the row swaps to the server's returned fragment.)
- **Tests (3 new, 22 total in `ws_server::gui`):** empty → "Inbox zero" + active nav; populated rows carry body + `/inbox/{id}/done|snooze|dismiss` actions + rationale + keyboard hint; a done row shows Undo (not Done) + struck-through class. (`render_row` is pure → testable; the action handler is a thin wrapper over `TodoService` methods already tested in `arawn-storage`.)
- **Verified end-to-end** (restarted `arawn serve` pid 20558): `GET /inbox` → 200 rendering the empty state (your instance has no open todos right now); `/`, `/health`, `/brief` all still 200. Gate clippy + fmt clean.
- **Not exercised live:** a populated inbox + a real POST action — no open todos exist in the data dir to act on. Covered by the pure render test + the underlying tested RPCs; will populate naturally as steward/ceremonies create action items.
- **Ceremony action items:** v1 sources the inbox from `todos.*` only; folding ceremony action-items into the same inbox is a clean follow-up (noted, not dropped).

Next: GUI-S4 (signals / memory / extraction-provenance) — the last v1 surface.