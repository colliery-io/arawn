---
id: daily-plugin-ceremony-trait-impl
level: task
title: "Daily plugin — Ceremony trait impl with gather + compose"
short_code: "ARAWN-T-0296"
created_at: 2026-05-16T14:00:00+00:00
updated_at: 2026-05-16T14:10:45.341862+00:00
parent: ARAWN-I-0041
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: ARAWN-I-0041
---

# Daily plugin — Ceremony trait impl

## Parent Initiative

[[ARAWN-I-0041]]

## Objective

Ship the daily ceremony plugin in `arawn-ceremonies` — a `Ceremony`
trait impl that runs the gather→compose pipeline on the existing
engine. Mirrors `RetroCeremony`'s shape (gather payload → LLM
compose → `ComposedItem` writes with citation_ids) but produces a
daily tablet rather than a retro one.

## Acceptance Criteria

## Acceptance Criteria

- [ ] New `crates/arawn-ceremonies/src/plugins/daily.rs` with
      `DailyCeremony { llm, model, calendar_source, attention_source }`.
- [ ] `Ceremony` trait impl:
      - `kind() = "daily"`
      - `period_key(now) = now.format("%Y-%m-%d")` (UTC date)
      - `default_schedule() = CronSchedule::local("0 7 * * MON-FRI")`
      - `gather()` builds a `GatherPayload` containing four sections'
        worth of source rows; see Technical Approach below.
      - `compose()` calls the LLM, parses JSON, returns `NewItem::Composed`
        rows with `citation_id` set from the gather payload.
- [ ] Compose prompt instructs the LLM to write into exactly four
      section keys: `calendar`, `todos`, `attention`, `alignment`.
- [ ] Every composed item carries a non-empty `citation_id` matching
      a gather payload row id. Engine's two-write-path contract from
      T-0282 fails the dispatch otherwise.
- [ ] In-crate UAT-style test with `MockLlmClient` (pattern from
      T-0287's `retro_uat.rs`): seeded calendar + signals + todos +
      priorities → run plugin → assert tablet generated, items in
      each of four sections, all carry valid citation_ids.
- [ ] Re-exported from `arawn-ceremonies::lib`.

## Implementation Notes

### Technical Approach

1. Gather payload shape (`#[derive(Serialize)]`):
   ```rust
   struct DailyGather {
       date: String,
       calendar_events: Vec<CalEvent>,       // from CalendarSource
       rolling_todos: Vec<TodoRow>,          // SELECT from ceremony_todos_rolling WHERE done_at IS NULL
       attention_signals: Vec<SignalRow>,    // from AttentionSource (since last daily generated_at)
       weekly_priorities: Vec<PriorityRow>,  // SELECT from ceremony_priorities for current iso_week
   }
   ```
2. `calendar_source` and `attention_source` are `Arc<dyn CalendarSource>` /
   `Arc<dyn AttentionSource>` traits defined in [[ARAWN-T-0297]] so the
   plugin doesn't take hard deps on arawn-feeds / arawn-projections.
   The plugin trait method signatures hand the plugin a `&ConnHandle`
   for the SQLite-backed gather queries.
3. Compose prompt: one structured user message containing
   `DailyGather` as JSON + system instruction asking for a JSON
   array of `{ section_key, body: { text }, citation_id }`. Same
   parsing approach as retro.
4. Citation grounding: include a registry of valid citation ids in
   the prompt so the LLM physically can't cite anything else; on
   parse, validate each `citation_id` is in the registry before
   constructing the `ComposedItem`.

### Dependencies

- Blocked by [[ARAWN-T-0297]] for the source traits — start by
  stubbing trivial test impls (e.g. `NoopCalendarSource`) so this
  task can land before the real adapters.
- Schema is V6, already shipped under [[ARAWN-T-0280]].

### Risk Considerations

- **Empty-gather days**: if all four sections are empty, the LLM
  may produce empty output. Treat that as "skip generation" (return
  `DispatchOutcome::Skipped`) rather than writing an empty tablet.
- **Prompt token bloat**: large calendar+attention payloads can
  blow context. Cap rows per section (sensible defaults: calendar
  ≤ 12, attention ≤ 10, todos ≤ 20, priorities ≤ 5); document the
  caps in the task body.

## Status Updates

### 2026-05-16 — daily plugin shipped

- New `crates/arawn-ceremonies/src/plugins/daily.rs` (513 lines)
  with `DailyCeremony` implementing `Ceremony`. period_key returns
  `YYYY-MM-DD`, default schedule `0 7 * * MON-FRI` local, no
  interactive actions, no pattern detectors.
- New `crates/arawn-ceremonies/src/plugins/gather_sources.rs`
  defining the `CalendarSource` + `AttentionSource` traits with
  `CalEvent` / `SignalRow` DTOs, plus three stub impls
  (`NoopCalendarSource`, `StaticCalendarSource`,
  `StaticAttentionSource`) for tests.
- Gather caps: calendar ≤ 12, attention ≤ 10, todos ≤ 20,
  priorities ≤ 5. Citation registry built from gather payload ids;
  compose validates each LLM-emitted `citation_id` is in the
  registry before constructing `NewItem::Composed`.
- Re-exports added in `lib.rs` for downstream binary wiring.
- Five in-crate tests pass: period_key shape, gather collects all
  four sections, compose rejects unknown/empty citations,
  end-to-end dispatch via `EngineDispatcher`. Full ceremonies
  suite (85 lib + 2 UAT) still passes.

Completed 2026-05-16.