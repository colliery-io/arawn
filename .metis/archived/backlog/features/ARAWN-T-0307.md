---
id: ceremony-tui-slash-commands-today
level: task
title: "Ceremony TUI slash commands — /today /week /retro"
short_code: "ARAWN-T-0307"
created_at: 2026-05-16T18:27:31.889657+00:00
updated_at: 2026-05-16T19:49:19.092891+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Ceremony TUI slash commands — `/today` `/week` `/retro`

## Objective

The three ceremony plugins ship LLM-judged end-to-end via the
agent tool surface, but the human user has no direct
slash-command path. Each initiative (I-0041, I-0042, I-0043)
deferred this for the agent-first UAT loop. This task closes the
gap: three TUI commands that render the current ceremony tablet
via the existing markdown renderer, with interactive bindings
for the diary (retro) and the priority confirmation flow
(weekly).

## Backlog Item Details

### Type
- [x] Feature - New functionality

### Priority
- [ ] P0
- [ ] P1
- [x] P2 — Medium. The agent path covers the same functionality;
  this is a UX shortcut for direct keyboard access.

### Business Justification
- **User Value**: typing `/today` is faster than chatting "show
  me today's brief," and the rendered markdown is denser than
  what the agent narrates back.
- **Effort Estimate**: M. Three commands + an interactive
  selection pattern + the existing markdown renderer reuse.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `/today` slash command in `arawn-tui` fetches the current
      daily tablet via `ceremonies.get_by_period`, fetches items
      via `ceremonies.list_items`, renders via
      `arawn_ceremonies::render::render_retro`-style helper
      (a new `render_daily` function — calendar/todos/attention/
      alignment sections).
- [ ] `/week` slash command: same shape for weekly with five
      sections + interactive priority confirm/reject keybinds
      when the tablet status is `open` (Monday morning flow).
      `space` confirms a candidate, `d` rejects, `a` opens an
      inline editor for `weekly_add_priority`.
- [ ] `/retro` slash command: existing `render_retro` already
      shipped; this command fetches + renders + offers an inline
      editor for the diary (`upsert_diary`).
- [ ] Read-only when the tablet's status is `reviewed` (or for
      `/week` Tue–Sun): no confirm/reject keybinds, no diary
      edit prompt.
- [ ] All three commands subscribe to the WS notice channel and
      auto-refresh on `CeremonyEvent::TabletGenerated` /
      `ItemUpdated` / `DiaryUpdated` / `PriorityConfirmed`.

## Implementation Notes

### Technical Approach

1. Slash command registration lives in `arawn-tui` —
   pattern-match on the slash-command dispatcher.
2. Each command opens a small modal-style screen (similar to
   how `/workstream show` renders today).
3. Markdown rendering reuses the existing pipeline; the daily
   + weekly renderers join `render_retro`. Put them in
   `arawn-ceremonies/src/render.rs` next to the retro one.
4. Interactive bindings only on `open` tablets — checked at
   render time.

### Dependencies

- Builds on the shipped engine + service + RPC surface
  (T-0292, T-0302) and renderer (T-0290).
- Independent of [[ARAWN-T-0306]].

### Risk Considerations

- **WS subscription churn**: each slash command opening a fresh
  WS subscription would leak handles. Subscribe once at TUI
  startup and broadcast events to whatever ceremony view is
  active.
- **Selection state for confirm/reject**: needs a small list
  widget. Either reuse an existing ratatui widget the project
  already has, or copy the pattern from
  `tag-promoter-cycle`'s ontology-picker.

## Status Updates

### 2026-05-16 — Phase 1 (read-only MVP) shipped

Scope split: Phase 1 (this slice) ships the three read-only
commands; Phase 2 (interactive priority confirm/reject + diary
editor + WS auto-refresh) deferred to a follow-up since the
agent tool surface already covers those flows.

- **Renderers in `arawn-ceremonies/src/render.rs`**:
  - `DailyView` + `render_daily` — four sections (calendar,
    todos, attention, alignment) with placeholder text per
    empty section + deduped footnotes.
  - `WeeklyView` + `render_weekly` — five sections, confirmed
    priorities render `[x]`, candidates `[ ]`, header shows
    whether priorities have been confirmed via
    `tablet.priorities_confirmed_at`.
  - Both re-exported from lib.rs.
- **TUI wiring**:
  - `arawn-ceremonies` added as dep of `arawn-tui`.
  - Three new `CommandResult` variants (`CeremonyShowToday`,
    `CeremonyShowWeek`, `CeremonyShowRetro`) + parse arms +
    suggestion-registry entries.
  - Event-loop handlers in `event_loop.rs` call
    `ceremonies.get_by_period` + `list_items` (+ `list_priorities`
    for weekly), build the view, render to markdown, push as
    `SystemMessage`. Null tablet → "No daily tablet for today —
    run `/agent daily_run`…" guidance.
- 9 new render tests pass (16 render tests total). arawn-tui
  155/155 green.

### Deferred to Phase 2

- Interactive priority confirm/reject keybinds on `/week`.
- Inline diary editor on `/retro`.
- WS event subscriptions for auto-refresh on
  `TabletGenerated` / `ItemUpdated` / `DiaryUpdated` /
  `PriorityConfirmed`.

A follow-up task should pick up these three items; the rendered
markdown surface from Phase 1 is the substrate they extend.

Phase 1 completed 2026-05-16.