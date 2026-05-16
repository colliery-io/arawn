---
id: ceremony-tui-phase-2-interactive
level: task
title: "Ceremony TUI Phase 2 — interactive priorities + diary + WS auto-refresh"
short_code: "ARAWN-T-0308"
created_at: 2026-05-16T19:49:30.659157+00:00
updated_at: 2026-05-16T19:49:30.659157+00:00
parent:
blocked_by:
  - ARAWN-T-0307
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Ceremony TUI Phase 2 — interactive surfaces

## Objective

T-0307 shipped read-only `/today` `/week` `/retro` slash
commands. Phase 2 closes the gap to full TUI parity with the
agent surface: keyboard-driven priority confirm/reject on
`/week`, an inline diary editor on `/retro`, and live refresh
on ceremony state changes.

## Backlog Item Details

### Type
- [x] Feature

### Priority
- [ ] P0
- [ ] P1
- [x] P2 — Medium. The agent tools already provide these
  flows; this is a UX speedup for keyboard-first users.

### Effort Estimate
- M. Three sub-features sharing common modal/widget plumbing.

## Acceptance Criteria

- [ ] **`/week` priority interaction** when tablet status is
      `open`:
      - Renders priorities as a selectable list (ratatui widget
        — reuse the tag-promoter ontology-picker pattern).
      - `space` toggles the candidate via
        `ceremonies.confirm_priority { item_id }`.
      - `d` rejects via `ceremonies.reject_priority { item_id }`.
      - `a` opens an inline text input → submits
        `ceremonies.add_priority { tablet_id, body, rationale }`.
      - `q` / `esc` exits the modal back to the chat surface.
      - Read-only when status != `open` (no keybinds shown).
- [ ] **`/retro` diary editor**:
      - When the tablet has no diary or status=`open`, render
        a "press `e` to write diary" hint.
      - `e` opens an inline multi-line editor (textarea widget).
      - `ctrl-s` saves via `ceremonies.upsert_diary { tablet_id,
        body }`, `esc` cancels.
      - When a diary already exists, `e` loads it for editing.
- [ ] **WS auto-refresh**:
      - TUI startup subscribes once to the existing notice
        broadcast channel for `CeremonyEvent` payloads.
      - On `TabletGenerated` / `ItemUpdated` / `DiaryUpdated`
        / `PriorityConfirmed`, the active ceremony view
        re-renders.
      - No churn — a single shared subscription, not per-modal.

## Implementation Notes

### Technical Approach

1. The read-only renderers from T-0307 are the substrate;
   wrap them in a `Modal` widget that overlays the chat
   surface (the workstream-show command already does this —
   look for `WorkstreamShowModal` or similar).
2. List selection: cursor-position state + an "active index"
   highlight in the rendered markdown. Easiest is keeping
   parallel data structures: the `WeeklyView` for render +
   `Vec<PriorityDto>` for keyboard navigation.
3. Text input for diary: ratatui's `tui-textarea` crate if
   already in the workspace; else a homegrown line buffer is
   fine for v1 (no syntax highlighting needed).
4. WS event channel: the server already broadcasts
   `CeremonyEvent`s. The TUI client needs to consume them via
   the existing notice channel (used for plugin-reload
   notices); add `CeremonyEvent` to the consumed event types.

### Dependencies

- Builds on [[ARAWN-T-0307]] (renderers + read-only command
  dispatch).
- Touches `arawn-tui` modal/event-loop code + the WS notice
  consumer.

### Risk Considerations

- **Modal lifecycle**: opening a modal while the agent is
  streaming text into chat could deadlock the event loop.
  Confirm the workstream-show modal handles this cleanly
  before mirroring its pattern.
- **Concurrent updates**: WS event arrives mid-edit. Default
  to "edit wins" — the user's local buffer isn't overwritten;
  reload only triggers if no local edit is in progress.

## Status Updates

*To be added during implementation*
