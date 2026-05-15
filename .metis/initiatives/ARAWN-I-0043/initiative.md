---
id: weekly-retro-ceremony-scheduled
level: initiative
title: "Weekly retro ceremony — scheduled Friday introspection mixing feedback + diary"
short_code: "ARAWN-I-0043"
created_at: 2026-05-15T12:25:33.234303+00:00
updated_at: 2026-05-15T12:25:33.234303+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: weekly-retro-ceremony-scheduled
---

# Ceremony engine + Friday retro (first plugin)

## Context

This initiative ships **two things together**: (a) the `arawn-ceremonies` engine that all scheduled-introspection ceremonies plug into, and (b) the Friday retro as the first concrete plugin built on that engine. Daily prep (I-0041) and weekly prep (I-0042) are the next two plugins; future user-defined ceremonies will eventually plug into the same engine via a config format (out of scope here, follow-on).

The engine framing matters because daily/weekly/retro share more than they differ: scheduling, artifact tables, the gather→compose→write contract, citation enforcement, the notification surface, and the RPC surface are all identical across all three. The differences (which queries to run, which prompt to use, whether to compute patterns, what interactive UX to attach) are per-ceremony concerns. Rather than three sibling initiatives copy-pasting plumbing, this initiative builds the engine once and demonstrates it with retro — the hardest of the three. If the engine handles retro, it handles daily and weekly trivially.

### Design decisions locked during initiative kickoff

1. **Scheduler: cloacina.** Reuse the in-process cloacina runtime that already drives feeds. No new scheduler.
2. **Transport: pure WebSocket RPC.** No HTTP server. Ceremony methods join the existing WS-RPC surface that `arawn-tui` already speaks; the HTTP shapes called out throughout this doc translate 1:1 into RPC method names (`ceremonies.get_today`, `ceremonies.patch_item`, `ceremonies.run`, etc.). State-change pushes ride the existing `EngineEvent` broadcast as new `ceremony.*` variants.
3. **Storage: shared `arawn.db`.** All `ceremony_*` tables land in `crates/arawn-storage/migrations/00NN_ceremonies.sql`. Same connection feeds, sessions, workstreams use. One transactional surface, one backup story.
4. **Citation enforcement: two write paths.** The engine exposes `write_composed_item(citation_id, …)` (LLM path; citation required, refuses without one) and `write_user_item(…)` (user path; no citation needed, used for freeform diary entries and user-added todos). Pattern items pass the pattern row id as their `citation_id`; the pattern row itself carries its source citations in `payload` JSON. Two-level citation chain stays grounded.

**Why retro is load-bearing among the three:** daily and weekly prep are forward-looking projections — relatively low risk because they summarize state the user can verify against their own calendar/inbox in minutes. The retro is harder: it looks backward and tries to tell a story about how the week went. Get it wrong and it becomes either generic ("you had a productive week!") or fabricated (citing things that didn't happen). Building the engine against retro's hard requirements (multi-week comparative data, pattern detection, citation enforcement, mixed LLM/user-authored content) ensures it can carry daily and weekly without future refactoring.

The retro mixes two modes:
- **Feedback** (assistant→user): "you scheduled 3 deep-work blocks but interrupted 2 with Slack threads in #incidents — pattern from last 3 weeks too." Grounded, comparative, occasionally uncomfortable.
- **Diary** (user→self, captured by assistant): user reflects in their own words; arawn structures and stores it; future retros and weekly preps read it back.

This dual mode is the core design challenge — keeping LLM-generated feedback honest *while* giving the user space to write their own honest reflection.

## Sequencing

**Ships first of the three ceremony initiatives, in two stages:**

1. **Engine + retro plugin stage (do first, before any of I-0041/I-0042):** ship the `arawn-ceremonies` engine (cloacina-backed scheduling, artifact tables in shared `arawn.db`, gather→compose→write contract, two-write-path citation enforcement, WS-RPC surface, WS `ceremony.*` events, notification routing) plus the **retro ceremony plugin** as the first concrete implementation. I-0041 and I-0042 then plug into the same engine as additional ceremony modules.
2. **Real-data operation stage (do last, after I-0041 and I-0042 have produced ≥4 weeks of real tablets):** turn retro on for the user. Validate patterns are recognizable against lived experience. This stage's exit criteria can only be checked after the other plugins have accumulated history.

This split lets the engine's shape get locked by retro's hardest requirements upfront. Daily/weekly plugins land later as small modules, not copy-pasted plumbing.

## Plugin contract (engine ↔ ceremony)

Each ceremony plugin is a Rust module implementing a trait roughly shaped:

```rust
trait Ceremony {
    fn kind(&self) -> &'static str;              // "daily" | "weekly" | "retro" | ...
    fn period_key(&self, now: DateTime) -> String;
    fn default_schedule(&self) -> CronSchedule;
    async fn gather(&self, ctx: &Ctx) -> GatheredFacts;
    async fn compose(&self, ctx: &Ctx, facts: GatheredFacts) -> Vec<NewItem>;
    fn interactive_actions(&self) -> &[InteractiveAction];  // confirm priorities, edit diary, etc.
    fn patterns(&self) -> Option<&dyn PatternDetector> { None }  // retro only
}
```

The engine owns the cron loop, transactional writes to `ceremony_*` tables in the shared `arawn.db`, citation enforcement via the two-write-path contract (LLM path requires `citation_id`; user-write path does not), WS-RPC method dispatch, `ceremony.*` event emission on the existing broadcast channel, and notification routing. Adding a new ceremony in Rust is "implement this trait, register it" — no schema changes, no RPC plumbing.

**User-facing ceremony definition format (TOML/YAML) is explicitly out of scope for v1.** Ship three plugins first, learn from real usage, then decide whether to expose a config surface in a follow-up initiative.

## Goals & Non-Goals

**Engine goals:**
- `arawn-ceremonies` crate exposes the plugin trait + the shared infrastructure (scheduling, artifact tables in shared `arawn.db`, gather→compose→write pipeline, two-write-path citation enforcement, WS-RPC surface).
- Adding a new ceremony in Rust is one module + registration — no schema, API, or scheduling work.
- Engine has zero ceremony-specific logic; everything ceremony-specific lives in plugin modules.

**Retro plugin goals (the first concrete ceremony):**
- Runs Friday afternoon (default: 16:00 local, on by default).
- Three sections, in order:
  1. **What happened** — grounded summary of the week from daily tablets + feed activity + calendar. Every claim cited.
  2. **Patterns the assistant noticed** — observations comparing this week to prior weeks (priorities-vs-completion ratio, time allocation drift, recurring interruption sources, workstream neglect).
  3. **Your reflection** — empty diary section the user fills in; arawn stores it verbatim.
- Retro reads back into future weekly preps: last retro's diary + patterns appear as context when proposing next week's priorities.
- Patterns rely on **multi-week comparative data** — fall back to "not enough history yet" rather than fabricating.

**Non-Goals:**
- User-facing ceremony definition format (TOML/YAML, "BYO ceremony"). Future initiative; not v1.
- Mood tracking, journaling prompts beyond a blank section, scoring/rating the week.
- Sentiment analysis of user diary.
- Sharing retros (these are private; surface is local files).
- Auto-coaching ("you should do X next week") — observations stay descriptive, prescriptions are for the user.

## Detailed Design

### Data model — the prerequisite

Retros read from SQLite, never from disk artifacts. The retro initiative defines the canonical ceremony schema that I-0041 and I-0042 then conform to:

- `ceremony_tablets`, `ceremony_sections`, `ceremony_items`, `ceremony_todos_rolling` — defined in I-0041 Data Model. The retro depends on `done_at` timestamps and `citation_id` columns being populated honestly.
- `ceremony_priorities` — defined in I-0042. Retro reads confirmed-vs-done counts here.
- Steward journal (existing) — proposals + accept/reject history.

New for I-0043:

```sql
CREATE TABLE ceremony_activity_rollup (
    iso_week TEXT NOT NULL,
    workstream TEXT NOT NULL,
    metric_key TEXT NOT NULL,         -- emails_sent | slack_threads_participated | meetings_attended | deep_work_hours | …
    value REAL NOT NULL,
    PRIMARY KEY (iso_week, workstream, metric_key)
);

CREATE TABLE ceremony_patterns_detected (
    id TEXT PRIMARY KEY,
    iso_week TEXT NOT NULL,
    pattern_key TEXT NOT NULL,        -- priority_completion_ratio | rollover_heat | workstream_neglect | …
    magnitude REAL NOT NULL,
    payload TEXT NOT NULL,            -- JSON: cited rows, comparison window, etc.
    surfaced_in_retro BOOLEAN NOT NULL
);

CREATE TABLE ceremony_diary (
    tablet_id TEXT PRIMARY KEY REFERENCES ceremony_tablets(id),
    body TEXT NOT NULL,
    written_at TEXT NOT NULL,
    word_count INTEGER NOT NULL
);
```

The activity rollup pipeline runs end-of-week (before retro generation), aggregating raw feed/calendar/tablet activity into the `ceremony_activity_rollup` table. Pattern detection then runs SQL over the rollup table — no per-week JSON files, no markdown parsing.

**Because this schema constrains the daily tablet format (I-0041) and priorities table (I-0042), it must be settled before either starts implementation.**

### Compose chain

Three-stage to enforce honesty:
1. **Gather** (deterministic SQL): query this week's daily tablets, weekly tablet (priorities + confirmation timestamp), activity rollup, steward journal entries, last N retros for comparative context. Output an in-memory `GatheredFacts` struct.
2. **Pattern detect** (deterministic stats, no LLM): SQL aggregations over `ceremony_activity_rollup` + `ceremony_priorities` + `ceremony_items` across the trailing comparison window. Each detected pattern inserts a row into `ceremony_patterns_detected` with cited source rows in `payload`.
3. **Compose** (LLM): given `GatheredFacts` + the detected pattern rows, the plugin calls `engine.write_composed_item(citation_id, …)` for each item it produces. The engine rejects calls without a `citation_id` — that is the load-bearing enforcement. The user-write path (`engine.write_user_item(…)`) skips the citation check; used only for `kind=freeform` diary entries and user-added todos.

### RPC additions (retro)

The retro plugin contributes these WS-RPC methods to the shared `ceremonies.*` namespace:

```
ceremonies.get_retro_current                                  → current retro tablet
ceremonies.get_retro { iso_week }                              → specific retro
ceremonies.upsert_diary { tablet_id, body }                    → upsert diary body
ceremonies.get_activity_rollup { iso_week }                    → rollup for the week
ceremonies.list_patterns { since_iso_week }                    → detected patterns history
```

`EngineEvent` variants emitted on the shared broadcast channel: `Ceremony::RetroGenerated`, `Ceremony::DiaryUpdated`, `Ceremony::PatternDetected`. The existing TUI WS subscriber forwards these to clients alongside chat events.

### Diary capture

Friday afternoon notification (server-side WS event). `/retro` TUI client:
1. fetches retro via `ceremonies.get_retro_current` RPC
2. renders "what happened" + "patterns" sections (from `ceremony_items`) and a blank diary editor
3. user writes markdown; on save, the TUI calls `ceremonies.upsert_diary { tablet_id, body }` which upserts the `ceremony_diary` row
4. server flips tablet status to `reviewed`

If the user never writes the diary by Sunday night, status auto-transitions to `unreviewed`. The diary table simply has no row for that tablet; future retros can detect "diary skipped 3 weeks running" via SQL count.

### Pattern catalog (initial set)

Concrete patterns to detect in v1:
- **Priority completion ratio**: % of weekly priorities marked done.
- **Rollover heat**: todos that rolled forward ≥3 days then either completed or were dropped.
- **Workstream neglect**: workstreams with prior activity that had ~0 this week.
- **Interruption hotspots**: deep-work calendar blocks overlapping with significant Slack/email activity.
- **Meeting drift**: meeting hours vs. trailing-4-week average.

This list expands over time but each pattern needs grounding data — don't add LLM-detected patterns.

### Bootstrap problem

For the first ~4 weeks after the user enables ceremonies, there is no multi-week history. Pattern detector returns "insufficient history" gracefully and the retro renders sections 1 (what happened) + 3 (diary) only. Section 2 (patterns) appears once enough weeks have accumulated.

## Alternatives Considered

- **Fully LLM-generated retro** — rejected. Too easy to fabricate; fabrications in a retrospective are worse than a missing retro because they retroactively rewrite the user's memory of the week.
- **No diary section (just feedback)** — rejected. Removes user voice; reduces the retro to surveillance.
- **Daily mini-retro instead of weekly** — rejected for v1. Weekly is the cadence the user explicitly asked for, and daily retro requires even better activity data.
- **Skip the patterns section in v1** — considered. Tradeoff: faster shipping vs. retro feeling shallow. Recommend keeping patterns but starting with just 2–3 from the catalog above, expanding over time.
- **Ship daily/weekly first then design retro** — rejected. Would mean refactoring daily tablet format once retro's data needs are clear. Doing schema design first costs little extra time and prevents a painful migration.

## Implementation Plan

Decompose during discovery → design. Rough shape:

**Stage 1 — engine + retro plugin (blocks I-0041 + I-0042):**

*Engine work:*
1. `arawn-ceremonies` crate scaffold + the `Ceremony` plugin trait + plugin registry.
2. Shared SQLite migrations for the full schema (tablets/sections/items/rolling-todos/priorities/activity-rollup/patterns/diary). I-0041 and I-0042 inherit this.
3. Shared cron + cloacina-backed workflow runner that dispatches to registered ceremony plugins.
4. Shared gather→compose→write pipeline with citation enforcement (no item written without `citation_id`).
5. Shared WS-RPC namespace (`ceremonies.*`) routed to ceremony plugins by `kind`; `EngineEvent::Ceremony(_)` variants on the broadcast channel.
6. Activity rollup pipeline — generic end-of-period aggregation into `ceremony_activity_rollup`. Used by retro now; available to any future ceremony.
7. Pattern detector framework — pluggable per-ceremony, generic infrastructure (write rows into `ceremony_patterns_detected` from SQL aggregations).

*Retro plugin work:*
8. Retro ceremony plugin module implementing the trait: Friday cron, per-week period_key, gather queries, compose prompt, pattern catalog (priority completion + rollover heat + workstream neglect to start), diary upsert action.
9. Retro-specific RPC methods (`ceremonies.upsert_diary`, `ceremonies.list_patterns`).
10. `/retro` TUI client (fetches via API, renders, saves diary via PUT).
11. Synthetic-data UAT: 4 weeks of synthetic tablet rows → run retro plugin → assert pattern rows + cited items + diary persistence + engine correctness independent of plugin specifics.

**Stage 2 — real-data operation (waits on I-0041 + I-0042 running):**
7. Weekly-prep integration: I-0042 reads last retro as context for next priorities (closes the loop).
8. Run on user's real data for ≥4 weeks; validate patterns surfaced are recognizable.

## Exit Criteria

Stage 1 — engine:
- [ ] `arawn-ceremonies` crate ships with `Ceremony` trait, plugin registry, shared cron runner, shared gather→compose→write pipeline.
- [ ] Full ceremony SQLite schema migrated; I-0041 and I-0042 inherit by registering a plugin module (no schema changes).
- [ ] Shared `ceremonies.*` WS-RPC namespace + `EngineEvent::Ceremony(_)` broadcast variants route to plugins by `kind`.
- [ ] Citation enforcement lives in the engine (compose path): no `ceremony_items` row written without `citation_id`.
- [ ] Activity rollup pipeline + pattern detector framework are engine-level — usable by future ceremonies, not retro-specific.
- [ ] Adding a fourth ceremony in Rust is "implement the trait + register" — demonstrated by a stub plugin in tests.

Stage 1 — retro plugin:
- [ ] Retro plugin produces a tablet on the Friday cron with cited `what_happened` items + at least 3 patterns triggered against synthetic 4-week data.
- [ ] Diary upsert via API persists verbatim in `ceremony_diary` and is readable from subsequent retros.
- [ ] Bootstrap path (no history) renders gracefully — pattern detector returns zero rows, retro renders without a patterns section.

Stage 2 (after I-0041 + I-0042 have accumulated ≥4 weeks of real data):
- [ ] At least 3 patterns from the catalog fire correctly against the user's real history.
- [ ] User confirms surfaced patterns are recognizable (not generic).
- [ ] Weekly prep (I-0042) shows last retro's diary + patterns as candidate-priority context.

## Context **[REQUIRED]**

{Describe the context and background for this initiative}

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- {Primary objective 1}
- {Primary objective 2}

**Non-Goals:**
- {What this initiative will not address}

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

{Delete if not a requirements-focused initiative}

### User Requirements
- **User Characteristics**: {Technical background, experience level, etc.}
- **System Functionality**: {What users expect the system to do}
- **User Interfaces**: {How users will interact with the system}

### System Requirements
- **Functional Requirements**: {What the system should do - use unique identifiers}
  - REQ-001: {Functional requirement 1}
  - REQ-002: {Functional requirement 2}
- **Non-Functional Requirements**: {How the system should behave}
  - NFR-001: {Performance requirement}
  - NFR-002: {Security requirement}

## Use Cases **[CONDITIONAL: User-Facing Initiative]**

{Delete if not user-facing}

### Use Case 1: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

### Use Case 2: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

## Architecture **[CONDITIONAL: Technically Complex Initiative]**

{Delete if not technically complex}

### Overview
{High-level architectural approach}

### Component Diagrams
{Describe or link to component diagrams}

### Class Diagrams
{Describe or link to class diagrams - for OOP systems}

### Sequence Diagrams
{Describe or link to sequence diagrams - for interaction flows}

### Deployment Diagrams
{Describe or link to deployment diagrams - for infrastructure}

## Detailed Design **[REQUIRED]**

{Technical approach and implementation details}

## UI/UX Design **[CONDITIONAL: Frontend Initiative]**

{Delete if no UI components}

### User Interface Mockups
{Describe or link to UI mockups}

### User Flows
{Describe key user interaction flows}

### Design System Integration
{How this fits with existing design patterns}

## Testing Strategy **[CONDITIONAL: Separate Testing Initiative]**

{Delete if covered by separate testing initiative}

### Unit Testing
- **Strategy**: {Approach to unit testing}
- **Coverage Target**: {Expected coverage percentage}
- **Tools**: {Testing frameworks and tools}

### Integration Testing
- **Strategy**: {Approach to integration testing}
- **Test Environment**: {Where integration tests run}
- **Data Management**: {Test data strategy}

### System Testing
- **Strategy**: {End-to-end testing approach}
- **User Acceptance**: {How UAT will be conducted}
- **Performance Testing**: {Load and stress testing}

### Test Selection
{Criteria for determining what to test}

### Bug Tracking
{How defects will be managed and prioritized}

## Alternatives Considered **[REQUIRED]**

{Alternative approaches and why they were rejected}

## Implementation Plan **[REQUIRED]**

{Phases and timeline for execution}