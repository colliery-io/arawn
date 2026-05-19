---
id: current-time-header
level: task
title: "Inject current local time into every agent turn's system prompt"
short_code: "ARAWN-T-0368"
created_at: 2026-05-19T18:55:47.326211+00:00
updated_at: 2026-05-19T18:55:47.326211+00:00
parent: ARAWN-I-0052
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#agent-loop"
  - "#phase/todo"


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

- [ ] Every agent turn's outbound system prompt starts with a
  `Current time: ...` line in the documented format.
- [ ] Local timezone honored; UTC is used only as a fallback
  when timezone lookup fails (rare).
- [ ] Unit test on the prompt assembler: given a fixed clock
  injection, the output starts with the expected line.
- [ ] Snapshot tests for the system prompt — if any exist —
  are updated.
- [ ] `angreal test unit` green. `angreal check workspace` green.

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
