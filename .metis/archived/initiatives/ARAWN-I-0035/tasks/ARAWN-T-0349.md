---
id: briefview-render-brief-composer
level: task
title: "BriefView + render_brief composer (daily + weekly tablet)"
short_code: "ARAWN-T-0349"
created_at: 2026-05-19T03:00:00+00:00
updated_at: 2026-05-19T12:20:45.539458+00:00
parent: ARAWN-I-0035
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0035
---

# BriefView + render_brief composer (daily + weekly tablet)

## Parent Initiative

[[ARAWN-I-0035]]

## Objective

Add a `BriefView` struct + `render_brief` composer to
`arawn-ceremonies::render` that takes today's daily tablet and
this week's weekly tablet and emits a single markdown document
shaped as:

```
# Brief — <date>

## Today
<daily tablet sections>

## This week
<weekly tablet sections>
```

This is the library half of I-0035 Phase 2. T-0354 wires it to
`/brief` and the empty-chat path. No service / RPC plumbing here
— pure rendering composition over data the ceremony layer
already produces.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `BriefView { daily: Option<DailyView>, weekly: Option<WeeklyView> }`
      added to `crates/arawn-ceremonies/src/render.rs`. Both
      fields optional so callers can render even when one tablet
      is missing.
- [ ] `pub fn render_brief(view: &BriefView) -> String` composes
      a single markdown document:
      - Top: `# Brief — <today's date>`
      - `## Today` section with the daily tablet's content
        (reuse `render_daily` under the heading; OR inline the
        section loop with `### ` headings so the daily renderer
        stays untouched — implementer's choice).
      - `## This week` section with the weekly tablet's
        content, same pattern.
- [ ] Missing-daily case: section reads
      `_(no daily tablet — run /day to generate)_`.
      Missing-weekly case: section reads
      `_(no weekly tablet — Monday's ceremony will produce one)_`.
      Both-missing case: render emits both placeholders under
      the date header. No panic; callers can always call
      `render_brief` regardless of tablet state.
- [ ] Daily section uses the latest available tablet's
      `period_key` — caller decides whether to pass today's or
      most-recent-Friday's tablet. The composer does NOT do
      date logic; it renders whatever it's given.
- [ ] Heading depth: the existing `render_daily` and
      `render_weekly` emit `## <section>` headers. When wrapped
      under `## Today` / `## This week`, those need to demote
      to `### <section>` so the hierarchy reads cleanly. Either:
      (a) pass a `depth_offset` parameter, or (b) post-process
      the wrapped output via a simple `## ` → `### ` regex on
      the inner content. (b) is fine and simpler.
- [ ] Unit tests in the same module:
      - `render_brief_with_both_tablets` — both sections render
        their content; date header present; heading depths
        correct (no `## Calendar` at the inner level).
      - `render_brief_missing_daily` — daily placeholder
        renders, weekly content renders.
      - `render_brief_missing_weekly` — weekly placeholder
        renders, daily content renders.
      - `render_brief_missing_both` — both placeholders render.
      - `render_brief_demotes_inner_headings` — verifies no
        `\n## ` in inner content (only the two top-level
        `## Today` / `## This week`).
- [ ] `angreal test unit` green. `angreal check workspace` green.

## Implementation Notes

### Technical Approach

1. **Use the existing renderers, don't duplicate them.** Wrap
   `render_daily(daily_view)` and `render_weekly(weekly_view)`
   in the appropriate top-level sections. Demote the inner
   `## ` headings to `### ` via a simple string replace on the
   returned strings.
2. **Date header.** Use the daily tablet's `period_key` if
   present; else the weekly tablet's; else today's UTC date
   formatted as `YYYY-MM-DD`. (`render_brief` may want a
   `now: DateTime<Utc>` parameter for testability — wire it
   through.)
3. **Public API:**
   ```rust
   pub struct BriefView {
       pub daily: Option<DailyView>,
       pub weekly: Option<WeeklyView>,
   }
   pub fn render_brief(view: &BriefView, now: chrono::DateTime<chrono::Utc>) -> String;
   ```
4. **Test data** can be hand-constructed `TabletDto` + `ItemDto`
   + `PriorityDto` values — mirror the existing tests in
   `render.rs` for shape.

### Dependencies

- `arawn-ceremonies::render` (the existing module).
- `chrono` (already in scope).

### Risk Considerations

- Heading-depth demotion via string replace is a coarse tool;
  a `## ` inside a code fence would be a false positive. The
  existing `render_daily` / `render_weekly` don't emit code
  fences, so this is safe today — but flag it in the doc
  comment so a future renderer change doesn't surprise the
  composer.
- Empty `items` lists already render with placeholder text in
  the daily/weekly renderers — propagating those through is
  fine. The composer's own placeholders only fire when the
  entire view is `None`.

## Status Updates

### 2026-05-19 — BriefView + render_brief shipped

- `BriefView { daily: Option<DailyView>, weekly: Option<WeeklyView> }`
  added to `arawn-ceremonies/src/render.rs`. Public.
- `pub fn render_brief(view, now) -> String` composes:
  - `# Brief — <date>` (uses `daily.period_key`, falls through
    to `weekly.period_key`, then to `now.format("%Y-%m-%d")`).
  - `## Today` wrapping `render_daily` output, with inner `## `
    headings demoted to `### ` via `demote_h2_to_h3`. Empty
    path: `_(no daily tablet — run /day to generate)_`.
  - `## This week` wrapping `render_weekly` output, same
    demotion. Empty path: `_(no weekly tablet — Monday's
    ceremony will produce one)_`.
- `demote_h2_to_h3` is a private helper. Doc-comment on
  `BriefView` flags the demotion's coarseness (string-replace,
  would over-fire on `\n## ` inside fenced code; today's
  renderers don't emit code fences, so safe).
- 5 new unit tests in `render::tests`: with-both, missing-daily,
  missing-weekly, missing-both, demote-inner-headings (verifies
  exactly two `\n## ` — Today, This week — and that inner
  sections show up as `### Today's calendar`, `### Priorities`,
  not `## …`).
- Test data uses hand-built `TabletDto` / `ItemDto` /
  `PriorityDto` values mirroring the file's existing fixtures.
- `cargo test -p arawn-ceremonies --lib` 108/0 (103 prior + 5
  new). `angreal check workspace` green.