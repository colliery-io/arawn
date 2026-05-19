---
id: inject-current-local-time-into
level: task
title: "Inject current local time into every agent turn's system prompt"
short_code: "ARAWN-T-0368"
created_at: 2026-05-19T18:55:47.326211+00:00
updated_at: 2026-05-19T22:23:07.579567+00:00
parent: ARAWN-I-0052
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#agent-loop"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0052
---

# Inject current local time into every agent turn's system prompt

## Objective

Prepend a single "Current time" line to every agent turn's
system context so the agent stops guessing what "today" is.
Eliminates a recurring failure mode where date-sensitive tool
calls (ceremony lookups, todo windows, scheduling) drift across
turns because the agent has no reliable now-reference.

Scoped here because the pinned-windows refactor in this
initiative depends on the agent passing correct dates to
ceremony tools. Independent of the rest of the initiative —
can land in parallel with [[ARAWN-T-0364]] and friends.

## Scope

- Single line at the top of the system prompt assembly:
  `Current time: 2026-05-19 14:32 PDT (Mon)`
- Format: `YYYY-MM-DD HH:MM ZZZ (Day)` in the user's **local**
  timezone (same source as the ceremony cron). Day-of-week is
  the short form (Mon/Tue/...).
- Re-computed every turn (cheap; the system prompt is
  re-assembled per turn anyway).
- Runs above any other section so it's the first thing the LLM
  sees.

## Acceptance criteria

- [x] Every agent turn's outbound system prompt starts with a
  `Current time: ...` line. Wiring: `query_engine.rs` calls
  `.current_time(chrono::Local::now())` on every turn's
  builder.
- [x] Local timezone honoured via `chrono::Local::now()` —
  no fallback needed since chrono pulls the system zone
  directly. Format generic over `Tz: chrono::TimeZone` so
  tests pin a specific zone.
- [x] Unit tests verify the line starts the prompt and that
  the provided zone is formatted correctly.
- [x] Snapshot test still passes (didn't reference the old
  `- Date:` line).
- [x] `angreal test unit` green. `angreal check workspace` green.

## Status Updates — 2026-05-19

Landed.

- `SystemPromptBuilder::current_time(now)` builder method
  takes any `DateTime<Tz>` and emits
  `Current time: YYYY-MM-DD HH:MM ZZZ (Day)` at priority 0
  (lands first in the sorted output).
- `query_engine.rs` wires `chrono::Local::now()` per turn.
- Environment section no longer emits `- Date:` — that info
  is now in the current_time header, in local zone instead of
  UTC. Regression-fenced by
  `environment_no_longer_emits_date_line`.

Tests added (3): `current_time_appears_at_the_top`,
`current_time_uses_provided_timezone`,
`environment_no_longer_emits_date_line`.

25 system_prompt tests pass (+3). Workspace + check green.

Ready for review.

## Implementation notes

- Likely lives in `crates/arawn-engine/src/system_prompt.rs` or
  wherever the per-turn assembly happens (search for the
  current prompt builder).
- Use a `Clock` trait or function pointer for time so tests
  can inject a fixed value; the prompt builder is already a
  good integration test surface.
- Don't add a separate "today's date" line — the timestamp
  carries it.

Parent: [[ARAWN-I-0052]].