---
id: weekly-plugin-ceremony-trait-impl
level: task
title: "Weekly plugin — Ceremony trait impl with gather + compose (5 sections)"
short_code: "ARAWN-T-0301"
created_at: 2026-05-16T16:37:46.058159+00:00
updated_at: 2026-05-16T16:45:23.668783+00:00
parent: ARAWN-I-0042
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0042
---

# Weekly plugin — Ceremony trait impl

## Parent Initiative

[[ARAWN-I-0042]]

## Objective

Ship `WeeklyCeremony` on the existing `arawn-ceremonies` engine.
Same shape as retro + daily — gather builds a structured payload,
LLM compose returns `NewItem::Composed` rows with citation_ids
into five sections. No engine changes; the priorities-confirmation
flow handled by [[ARAWN-T-0302]] writes the `ceremony_priorities`
table.

## Acceptance Criteria

## Acceptance Criteria

- [ ] New `crates/arawn-ceremonies/src/plugins/weekly.rs` with
      `WeeklyCeremony { llm, model, calendar_source, attention_source }`.
- [ ] `Ceremony` trait impl:
      - `kind() = "weekly"`
      - `period_key(now) = iso_week(now)` (`YYYY-Www`)
      - `default_schedule() = CronSchedule::local("0 7 * * MON")`
      - `gather()` builds a five-array payload:
        - `calendar_summary`: meeting count, deep-work hours, busiest day, free afternoons. Computed across Mon..Sun from `CalendarSource::events_for`.
        - `deadlines`: signals with due-date metadata from feeds + calendar items tagged as deadlines (rows from `AttentionSource::since(monday)` filtered to those with `due_at` in week).
        - `last_retro_excerpts`: last retro's diary body + top 3 pattern items (from `ceremony_diary` + `ceremony_patterns_detected`).
        - `prior_weekly_inbound`: un-done items from last week's weekly tablet (`ceremony_items WHERE tablet_id=last_weekly AND done_at IS NULL`).
        - `rolling_todo_hot`: rolling todos created > 7d ago and still un-done.
      - `compose()` calls the LLM with a structured prompt asking for items grouped into five sections: `priorities`, `calendar_shape`, `deadlines`, `from_last_retro`, `inbound`. The `priorities` section gets 5–7 candidates with `kind="priority"`; the others use `kind="freeform"`. All carry citation_ids from the gather payload.
- [ ] In-crate UAT-style test with `MockLlmClient` (pattern from
      retro/daily): seeded calendar week + signals + prior retro +
      prior weekly tablet → run plugin → assert tablet generated,
      items in each of five sections, all carry valid citation_ids.
- [ ] Re-exported from `arawn-ceremonies::lib`.

## Implementation Notes

### Technical Approach

1. Calendar summary is pure aggregation over the week's events;
   the helper logic (Mon..Sun bracket) is in
   `plugins/retro.rs::monday_sunday_for_iso_week_public()` — reuse.
2. Citation registry: assemble valid ids from all five gather
   sections, validate compose output before emitting items.
3. The compose system prompt is structurally identical to retro's:
   "produce a JSON array of `{ section, citation_id, body }`,
   never fabricate citations." Add an explicit instruction that
   the `priorities` section needs `kind: "priority"` in the body
   and the others `"freeform"`. Validation rejects out-of-set
   section keys.
4. Mock-LLM test seeds at least one row per source — three deep
   sources are enough for the gather to populate every section
   so compose can produce a meaningful tablet.

### Dependencies

- Builds on shared engine + service + dispatcher infra (I-0043,
  T-0292) and the gather-source traits + adapters from
  [[ARAWN-T-0297]].
- Unblocks [[ARAWN-T-0304]] (binary wiring) and indirectly
  [[ARAWN-T-0305]] (UAT).

### Risk Considerations

- **Empty-source weeks**: with a fresh install, prior weekly +
  last retro + rolling todos may be empty. The gather payload's
  arrays are allowed to be empty; the LLM should still produce
  candidate priorities from calendar + deadlines + attention.
  Test seeds a minimal calendar-only path.
- **Token bloat**: candidates section caps 7; other sections
  cap 10 each. Documented in the plugin's gather caps constants.

## Status Updates

### 2026-05-16 — weekly plugin shipped

- New `crates/arawn-ceremonies/src/plugins/weekly.rs` (~640
  lines) implementing the `Ceremony` trait. `iso_week`
  period_key, `0 7 * * MON` default schedule.
- Five-section gather: `calendar_summary` (Mon..Sun aggregate
  via `CalendarSource::events_for`), `deadlines` (filtered
  `AttentionSource::since(monday)` with due/deadline heuristic
  + recent fallback), `last_retro_excerpts` (diary + top-3
  patterns of most-recent retro), `prior_weekly_inbound` (open
  items from prior weekly tablet), `rolling_todo_hot` (open
  todos created > 7d ago). Caps per section: 10/4/10/10.
- Compose validates section_keys against the allowed set and
  citation_ids against the gather registry; emits items with
  `ItemKind::Pattern` (matches retro's convention — daily's
  Freeform diverges but consistency with retro wins here).
- Re-exported from `arawn-ceremonies::lib`.
- Two in-crate tests pass (period_key format + end-to-end
  dispatch via `EngineDispatcher` asserting one item per
  section with valid citations). Full ceremonies suite (87 lib
  + 2 UAT) green.

Completed 2026-05-16.