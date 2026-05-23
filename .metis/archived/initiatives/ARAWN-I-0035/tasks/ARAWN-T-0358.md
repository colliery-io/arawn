---
id: phase-3-t-c-right-pane-action
level: task
title: "Phase 3 T-C — right-pane action items section"
short_code: "ARAWN-T-0358"
created_at: 2026-05-19T14:00:00+00:00
updated_at: 2026-05-19T14:20:38.772335+00:00
parent: ARAWN-I-0035
blocked_by: [ARAWN-T-0356]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0035
---

# Phase 3 T-C — right-pane action items section

## Parent Initiative

[[ARAWN-I-0035]]

## Objective

Populate the lower portion of the right pane (from
[[ARAWN-T-0356]]) with a persistent list of today's action
items — the items in the daily tablet's `attention` section
plus any open rolling todos.

Read-only for Phase 3. Mutation (mark done, snooze, dismiss)
is out of scope here — file as backlog when needed.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] **`render_dashboard_actions(app, frame, area)`** function
      in `arawn-tui::render` (or the new dashboard sub-module).
- [ ] Data source: `app.daily_view.items` filtered to
      `section_key == "attention"`. Items appear in the
      tablet's natural order (already sorted by ordinal).
- [ ] Optional secondary source: items from the `todos`
      section (carried-over rolling todos). Renders below the
      attention list under a `─ Carried over ─` separator. Skip
      this if it bloats the pane; the brief itself already shows
      todos.
- [ ] Each row renders as a checkbox + truncated body text:
      `☐ Reply to Alice (RFC-0042)`. Read-only — the checkbox
      doesn't toggle on click in Phase 3.
- [ ] **Empty state:** no daily tablet cached OR attention
      section empty → render `_(no action items)_` in
      `theme::SUBTEXT0`. The header
      `## Action items` stays visible regardless.
- [ ] **Width awareness.** Pane width ~28 cols minus checkbox
      glyph (2 cols) minus left padding (1 col) = ~25 cols of
      text. Truncate with `…` via `unicode_width`.
- [ ] **Overflow.** When more attention items exist than the
      pane has rows, render the first N − 1 items plus a
      `… +K more` footer line. N is computed from the available
      area.height.
- [ ] **Scroll.** Not needed in Phase 3 — when more items than
      fit, the overflow footer suffices. File scroll as a
      backlog enhancement if the list regularly overflows.
- [ ] **Unit tests:**
      - `dashboard_actions_renders_attention_items` — three
        attention items render as three checkbox rows.
      - `dashboard_actions_empty_state_no_tablet`
      - `dashboard_actions_empty_state_no_attention_items`
      - `dashboard_actions_truncates_long_titles`
      - `dashboard_actions_overflow_footer` — more items than
        fit produces `… +K more` line.
- [ ] **Snapshot test:**
      `snapshot_dashboard_actions_with_items` — three-pane
      layout with three attention items + one carried-over
      todo.
- [ ] `angreal test unit` green. `angreal check workspace` green.

## Implementation Notes

### Technical Approach

1. **Hooks into T-0356's pane structure.** The right pane is
   split vertically: T-B (brief) on top, T-C (actions) on
   bottom. The split ratio is something like brief gets up to
   12 rows, actions get the remainder.
2. **Filter.** `items_in_section(&items, "attention")` mirrors
   the existing helper in `arawn-ceremonies::render`. Could
   reuse if exported; otherwise re-implement locally.
3. **Body text extraction.** Same pattern as
   `render_item_bullet`: try `body.get("text").and_then(|v|
   v.as_str())`; fall back to JSON serialization.
4. **Rule-driven only.** Per the Phase 3 decomposition
   discussion, the agent-curated layer is out of scope and
   filed as a separate (deferred) initiative. All items shown
   here come from the ceremony's signal extraction.

### Dependencies

- [[ARAWN-T-0356]] — pane structure.
- [[ARAWN-T-0357]] — sibling pane section; vertical split
  inside the right pane is coordinated.
- `arawn-ceremonies::ItemDto` (already accessible via the
  cached `DailyView`).

### Risk Considerations

- "Attention" can be a wide-ranging section after signal
  extraction; on a busy day there might be 10+ items. The
  overflow footer handles this gracefully but means some items
  are hidden — a `/brief` invocation surfaces them all. That's
  the right escape hatch for Phase 3.
- Body text shape variance: structured items might have
  `body.summary`, `body.subject`, `body.text` — pick the most
  likely field per item kind. Worst case shows the JSON
  fallback, which is ugly but not wrong.

## Status Updates

### 2026-05-19 — Action items section shipped

- **`render_dashboard_actions(app, frame, area)`** replaces the
  T-0357 placeholder in the bottom portion of the dashboard pane.
- **Data sources** — both filtered out of `app.daily_view.items`:
  - Primary: `section_key == "attention"` (rule-driven attention
    items from ceremony signal extraction).
  - Secondary: `section_key == "todos"` (carried-over rolling
    todos), rendered below a `─ Carried over ─` separator when
    both sections have content.
- **Row rendering.** `format_action_row` extracts the title from
  `body.text`, `body.summary`, or `body.subject` (in that
  preference order); falls back to JSON serialization for
  unknown shapes. Width-aware truncation via
  `crate::width::truncate_display`. Rows render as `☐ <title>`.
- **Budget logic** (`push_action_rows` helper) handles the
  overflow case: when the row budget can't fit every item,
  render `budget - 1` items + a `… +K more` footer line. When
  carried-over rolling todos are also present, the attention
  section reserves ~2 rows for the carried section so it stays
  visible (squeezing attention's overflow rather than dropping
  carried entirely).
- **Empty states.** No daily view → header + `(no action items)`.
  Daily view cached but attention + todos both empty → same
  placeholder. Header stays visible in both cases so the user
  knows where the section *would* render.
- **Read-only.** Per the Phase 3 design discussion, mutation
  (mark done / snooze / dismiss) and per-row keyboard focus are
  out of scope. File as backlog if the read-only surface proves
  insufficient.
- **Unit tests (6 new):**
  - `dashboard_actions_renders_attention_items` — three
    attention items surface in the rendered buffer.
  - `dashboard_actions_empty_state_no_tablet` — None →
    placeholder.
  - `dashboard_actions_empty_state_no_attention_items` — both
    sections empty → placeholder.
  - `dashboard_actions_truncates_long_titles` — `…` appears on
    overlong rows.
  - `dashboard_actions_overflow_footer` — many items + small
    pane → `+K more` footer.
  - `dashboard_actions_carried_over_separator` — attention +
    todos both present → separator + carried rows render.
- **Snapshot test:** `snapshot_dashboard_actions_with_items`
  captures the three-pane layout with three attention items +
  one carried-over todo (date line, separator, all rendered as
  expected).
- 18 existing snapshots re-baselined for the placeholder →
  real-section transition (`(coming in T-0358)` → `(no action
  items)`).
- `cargo test -p arawn-tui --lib` 199/0 (192 prior + 7 new).
  `angreal check workspace` green.