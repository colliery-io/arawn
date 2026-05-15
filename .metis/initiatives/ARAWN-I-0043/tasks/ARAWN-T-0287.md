---
---
id: retro-plugin-friday-cron-gather
level: task
title: "Retro plugin — Friday cron, gather queries, compose chain with hint:medium"
short_code: "ARAWN-T-0287"
created_at: 2026-05-15T23:45:39.140814+00:00
updated_at: 2026-05-15T23:45:39.140814+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0043
---

# Retro plugin — gather + compose chain

## Goal
The first concrete `Ceremony` implementation. Runs Friday 16:00 local by default. Gathers this week's daily tablets + activity rollup + last N retros; calls pattern detectors; composes the `what_happened` and `patterns` sections.

## Reference
I-0043 Retro plugin goals + Compose Chain.

## Acceptance
- New `RetroCeremony` struct implementing `Ceremony`.
- `kind() = "retro"`, `period_key()` = ISO week (e.g. `2026-W20`), `default_schedule()` = `"0 16 * * FRI"` local.
- `gather()`:
  - daily tablets for the week (Mon–Fri)
  - this week's rollup (T-0285)
  - last 3 retros (for comparative context)
  - confirmed weekly priorities + their done_at status
- `compose()`:
  - LLM call via `hint:medium`, gated through `arawn_llm::gate::acquire_local()`, recorded as `ceremony.retro.compose`
  - prompt enforces "every claim cites a gather payload row"
  - writes items to `section_key = "what_happened"` with `write_composed_item` (citation required)
  - patterns section writes one composed item per `DetectedPattern` row using the pattern row id as citation
- `patterns()` returns the catalog (T-0288).
- Bootstrap fallback: when history < 2 weeks, skips patterns section entirely (still writes `what_happened` + an empty diary placeholder).
- Tests with `MockLlmClient`: assert sections + citations + bootstrap path.

## Out of scope
Pattern catalog v1 — T-0288. Diary capture — T-0289. TUI client — T-0290.
