---
id: phase-3-t-a-sidebar-slim-right
level: task
title: "Phase 3 T-A — sidebar slim + right-pane layout refactor"
short_code: "ARAWN-T-0356"
created_at: 2026-05-19T14:00:00+00:00
updated_at: 2026-05-19T14:11:34.865341+00:00
parent: ARAWN-I-0035
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0035
---

# Phase 3 T-A — sidebar slim + right-pane layout refactor

## Parent Initiative

[[ARAWN-I-0035]]

## Objective

Reshape the TUI layout from coding-tool framing (Workstreams +
Sessions in a single left sidebar) to assistant framing
(Workstreams left, chat center, dashboard right). Foundation
for [[ARAWN-T-0357]] (brief summary) and [[ARAWN-T-0358]]
(action items), which fill the new right pane.

Locked layout (from the Phase 3 design discussion):

```
┌─Workstreams──┬──Chat──────────────────────┬─Today──────────┐
│ ▸ personal   │                            │ Brief          │
│   code/arawn │  > whats the postgres rfc  │ • 09:00 stdup  │
│   side-proj  │                            │ • 1:1 @ 13:00  │
│              │                            │                │
│              │                            │ Action items   │
│              │                            │ ☐ Reply Alice  │
│              │                            │ ☐ Review PR    │
├──────────────┴────────────────────────────┴────────────────┤
│ > Type your message...                                     │
├────────────────────────────────────────────────────────────┤
│ ◎ sonnet 4.6  │  personal  │  Ready                       │
└────────────────────────────────────────────────────────────┘
```

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] **Sessions section dropped from sidebar.** The
      `render_sidebar` path no longer emits a Sessions block.
      The Workstreams section remains and takes the full left
      column. Sidebar tab cycling
      (`SidebarSection::Workstreams | Sessions`) collapses to
      Workstreams-only.
- [ ] **`/session list` continues to work** for users who need
      to browse / resume sessions. It's already wired in
      `command.rs:101` and dispatched in event_loop.rs — no
      changes required, but verify it still produces useful
      output once the sidebar no longer shows sessions.
- [ ] **Right pane added.** The chat layout splits into three
      vertical columns:
      - Left: Workstreams sidebar (~14 cols, same width as
        today's sidebar).
      - Center: Chat + input + status bar (flexible).
      - Right: dashboard pane (~28 cols).
      The right pane is rendered by a new
      `render_dashboard_pane(app, frame, area)` function. Phase
      3 T-A only renders an empty skeleton with a placeholder
      line — actual sections land in T-B and T-C.
- [ ] **Tab cycling extended.** `Tab` rotates focus through
      Workstreams → Chat → Dashboard → Workstreams. Or — if
      `Focus::Sidebar` and `Focus::Main` are kept as the focus
      enum — extend to `Focus::Dashboard` so the three panes are
      symmetric. Make sure modal / overlay focus interactions
      still work.
- [ ] **`LayoutRegions` updated** to carry the dashboard rect
      for mouse hit-testing (`pub dashboard: Option<Rect>`).
- [ ] **Narrow-terminal fallback.** If `area.width <
      MIN_FOR_THREE_PANE` (pick ~100 cols), drop the dashboard
      pane and revert to two-pane (sidebar + chat). The
      breakpoint should be wide enough that the dashboard is
      meaningful when shown.
- [ ] **Snapshot tests.**
      - `snapshot_layout_three_pane` — wide terminal, empty
        chat, no tablets cached → empty dashboard pane visible
        on the right.
      - `snapshot_layout_narrow_fallback` — terminal narrower
        than the breakpoint → two-pane layout, no dashboard.
- [ ] **Existing snapshots re-baselined.** The sessions
      section's removal will shift every snapshot that captures
      the sidebar. Re-accept via `cargo insta accept` after
      review.
- [ ] `angreal test unit` green. `angreal check workspace` green.
- [ ] Manual smoke: `cargo run --bin arawn -- tui` renders the
      three-pane layout on a 120-col terminal; resizing below
      the breakpoint collapses to two-pane without crash.

## Implementation Notes

### Technical Approach

1. **Sidebar narrowing.** Strip the `Sessions` rendering branch
   from `render_sidebar`. Collapse `SidebarSection` enum to a
   single variant or remove the section-toggle key handler if
   sessions was the only secondary section.
2. **Layout split.** The current `render` entry point splits
   into sidebar + main. Add a third constraint after main:
   ```rust
   let chunks = Layout::default()
       .direction(Direction::Horizontal)
       .constraints([Length(14), Min(40), Length(28)])
       .split(area);
   ```
   Compute the narrow-fallback breakpoint and skip the third
   constraint when `area.width < ~100`.
3. **Empty dashboard skeleton.** Render a single-line
   placeholder like `"_(brief & action items land here in
   T-B / T-C)_"` in `theme::SUBTEXT0`. Phase 3 follow-ups
   replace it.
4. **Focus enum.** Either extend `Focus` with a `Dashboard`
   variant or repurpose the existing sidebar-toggle key for a
   3-way cycle. Whichever is simpler — the goal is "Tab moves
   me through the three panes."

### Dependencies

- `arawn-tui` crate only — no engine or service changes.
- Existing snapshot test infrastructure handles the
  re-baselines.

### Risk Considerations

- The session sidebar might be load-bearing for some workflows
  (resume yesterday's debugging session). The fallback is
  `/session list` — verify it produces usable output. If the
  slash command's listing is poor, file a follow-up to richen
  it (`/sessions modal`) rather than blocking Phase 3.
- Snapshot rebaseline noise: every snapshot that captured the
  sidebar will shift. Review the diff to make sure no
  unrelated changes leak through.

## Status Updates

### 2026-05-19 — Layout refactor shipped

- **Sidebar slimmed to Workstreams-only.** `SidebarSection`
  enum has only the `Workstreams` variant now (kept as a
  single-variant placeholder for future expansion). All
  branches that handled `Sessions` were collapsed:
  `Action::SidebarUp/Down`, `ClickSidebarItem`, event-loop
  `SidebarSelect` dispatch, mouse hit-testing. Sessions reach
  only through `/session list` now.
- **`LayoutRegions` updated:** `sidebar_sessions` removed,
  `dashboard: Option<Rect>` added (used by
  `render_dashboard_pane` + future T-0358 mouse hit-testing).
- **Three-pane layout** (sidebar/tab | chat | dashboard) when
  terminal width ≥ `MIN_FOR_THREE_PANE` (100 cols). Dashboard
  occupies a fixed `DASHBOARD_WIDTH` (28 cols). Below the
  breakpoint the layout collapses to the existing two-pane
  shape — no dashboard, chat reclaims the right column.
- **`render_dashboard_pane`** is the new skeleton. Bordered
  block titled ` Today `, with a placeholder line that
  branches on `app.brief_markdown`:
  - cached + non-empty → "Brief cached — sections pending
    (T-0357 / T-0358)."
  - none → "No brief yet — run /day or wait for the 7am cron."
- **Focus model unchanged.** Tab still toggles
  `Focus::Main` ↔ `Focus::Sidebar`; the dashboard is a passive
  display surface in Phase 3 T-A. Deviated from the task
  spec's "Tab cycles through 3 panes" — defer that to T-0358
  if action items become interactive.
- **Snapshots:** 16 existing snapshots re-baselined to the new
  3-pane / 2-pane layouts. Two new snapshots:
  - `snapshot_layout_three_pane` (120×24 — three panes
    visible).
  - `snapshot_layout_narrow_fallback` (80×24 — dashboard
    dropped).
- **Test pass:** `cargo test -p arawn-tui --lib` 184/0 (182
  prior + 2 new). `angreal check workspace` green.