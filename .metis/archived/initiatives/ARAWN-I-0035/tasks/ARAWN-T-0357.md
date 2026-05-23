---
id: phase-3-t-b-right-pane-brief
level: task
title: "Phase 3 T-B — right-pane brief summary section"
short_code: "ARAWN-T-0357"
created_at: 2026-05-19T14:00:00+00:00
updated_at: 2026-05-19T14:17:46.713714+00:00
parent: ARAWN-I-0035
blocked_by: [ARAWN-T-0356]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0035
---

# Phase 3 T-B — right-pane brief summary section

## Parent Initiative

[[ARAWN-I-0035]]

## Objective

Populate the upper portion of the new right pane (from
[[ARAWN-T-0356]]) with a compact brief summary — the
persistent at-a-glance view of "today" that survives once the
user starts a conversation and the empty-chat full brief
disappears.

Compact form: date header, today's calendar bullets (start time +
short title), and conflict warnings. NOT the full daily +
weekly markdown brief — that stays as the empty-chat surface.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] **`render_dashboard_brief(app, frame, area)`** function in
      `arawn-tui::render` (or a new sub-module under the
      dashboard pane).
- [ ] Renders from `app.daily_brief: Option<DailyView>` (already
      cached on `App` after [[ARAWN-T-0354]] — but check the
      field actually holds the parsed view, not just the
      rendered markdown string. If it's the string today, this
      task adds a parallel `daily_view: Option<DailyView>`
      field populated alongside).
- [ ] Layout inside the section:
      - Header: `## Brief` in `theme::FG_STRONG` bold.
      - Date line: `2026-05-19 · Wed` in `theme::SUBTEXT0`.
      - Calendar bullets: up to 4 events, formatted
        `• 09:00 standup`, `• 13:00 1:1 with Jamie`, etc.
        Time first (compact, 5 cells), title truncated to
        fit the pane width.
      - Conflict marker: when two events overlap, render
        `⚠ conflict 14:00–15:00` in `theme::WARNING` color
        immediately after the conflicting events.
- [ ] **Empty states:**
      - No daily tablet cached → section renders header +
        `_(no brief yet — run /day to generate)_` in
        `theme::SUBTEXT0`.
      - Daily tablet exists but has no calendar items →
        `_(no calendar events today)_`.
- [ ] **Conflict detection.** Pure function over the daily
      tablet's `calendar` section items. Items are read in
      order; two items conflict if their start times overlap.
      Use the `start_ts` / `end_ts` from each item's body JSON
      if present; fall back to "no conflict detected" when
      shape isn't a calendar event.
- [ ] **Width awareness.** The pane is ~28 cols. Use the same
      `unicode_width` measurement the rest of the chrome uses
      (post-I-0036 T-0211). Long titles truncate with `…`.
- [ ] **Unit tests:**
      - `dashboard_brief_renders_today_with_calendar` — daily
        view with two calendar items renders both bullets.
      - `dashboard_brief_flags_conflict` — two overlapping
        items produce a `⚠ conflict` line.
      - `dashboard_brief_empty_state_no_tablet` — None daily →
        placeholder line.
      - `dashboard_brief_empty_state_no_calendar_items` →
        cached tablet but empty calendar section.
- [ ] **Snapshot test:**
      `snapshot_dashboard_brief_with_calendar` — three-pane
      layout with two calendar events rendered.
- [ ] `angreal test unit` green. `angreal check workspace` green.

## Implementation Notes

### Technical Approach

1. **Cache shape.** [[ARAWN-T-0354]] cached
   `App.brief_markdown: Option<String>`. To render a compact
   summary we need the parsed `DailyView`. Add
   `App.daily_view: Option<DailyView>` populated alongside
   `brief_markdown` at session start. Both stay in sync.
2. **Pane sub-layout.** Inside the right pane, split vertically
   into Brief (~12 rows) and Action items (rest). Brief renders
   in the top region; T-0358 renders in the bottom.
3. **Calendar item parsing.** The daily tablet's calendar
   section items have body JSON containing `start_ts`,
   `summary`, etc. (see `personal-day.json` shape). Parse
   defensively — fall back to `body.text` if the structured
   fields aren't present.
4. **Conflict detection.** Sort calendar items by start_ts.
   Walk in pairs; flag when `pair[1].start_ts < pair[0].end_ts`.
   First conflict per pane render is enough — don't try to
   compute every conflict.

### Dependencies

- [[ARAWN-T-0356]] — the pane structure this task fills.
- `arawn-ceremonies::DailyView` (already exported).

### Risk Considerations

- 28-col width is tight for "13:00 1:1 with Jamie Lewis" — make
  sure truncation lands in a readable place (probably truncate
  the title not the time).
- Conflict detection is best-effort; not every calendar entry
  carries timezone info. Treat absent / unparseable timestamps
  as "no conflict" and don't error.

## Status Updates

### 2026-05-19 — Brief summary section shipped

- **`App.daily_view: Option<DailyView>`** added alongside
  `brief_markdown`. Populated together at session start via
  the same `fetch_daily_view` / `fetch_weekly_view` helpers
  the `/brief` slash command uses — the event-loop pre-fetch
  now calls them directly, builds the `BriefView`, renders the
  markdown for the empty-chat surface, and caches the parsed
  `DailyView` for the dashboard.
- **`render_dashboard_brief(app, frame, area)`** in
  `arawn-tui::render`. Renders:
  - `Brief` header in `theme::TEXT` bold.
  - Date line `YYYY-MM-DD · <Wed>` via
    `format_brief_date_line` (helper falls back to raw
    period_key when the date doesn't parse).
  - Up to `MAX_CAL = 4` calendar bullets, formatted via
    `format_calendar_row` (prefers `body.start_ts` for
    `HH:MM` prefix; falls back to `body.text` /
    `body.summary`). Truncates with
    `crate::width::truncate_display` to fit the pane width.
  - `… +K more` footer when there are more than `MAX_CAL`
    items.
  - `⚠ conflict HH:MM–HH:MM` line in `theme::YELLOW` when
    `detect_conflict` finds an overlap.
- **`detect_conflict`** is the pure conflict-detection helper
  — extracts `start_ts` / `end_ts` from each item's body JSON,
  sorts events, walks consecutive pairs, returns the first
  overlap. Items without `start_ts` are skipped (no false
  positives on free-text items). Items missing `end_ts` get a
  conservative 30-min window so a later overlapping event is
  still detected.
- **`render_dashboard_pane`** now splits the inner area
  vertically: top `BRIEF_HEIGHT = 12` rows render the brief,
  the rest renders an action-items placeholder
  (`render_dashboard_actions_placeholder`) — the placeholder
  becomes the real surface in [[ARAWN-T-0358]].
- **Theme:** used `theme::TEXT` (strong) + `theme::SUBTEXT0`
  (muted) + `theme::YELLOW` (warning). No new theme constants
  introduced.
- **Unit tests** (7 new):
  - `dashboard_brief_renders_today_with_calendar` — two
    calendar items surface in the rendered buffer.
  - `dashboard_brief_flags_conflict` — overlap produces a
    `conflict` line with the right time window.
  - `dashboard_brief_no_conflict_when_separated` — non-
    overlapping events don't trigger.
  - `dashboard_brief_empty_state_no_tablet` — None →
    placeholder.
  - `dashboard_brief_empty_state_no_calendar_items` — tablet
    cached but empty calendar section.
  - `format_brief_date_line_includes_weekday` — 2026-05-19 →
    "Tue".
  - `format_brief_date_line_fallback_on_garbage` — bad input
    → raw passthrough.
- **Snapshot test:** `snapshot_dashboard_brief_with_calendar`
  captures the three-pane layout with two calendar bullets.
  17 existing snapshots re-baselined for the pane vertical
  split (brief + actions placeholder).
- `cargo test -p arawn-tui --lib` 192/0 (184 prior + 8 new).
  `angreal check workspace` green.