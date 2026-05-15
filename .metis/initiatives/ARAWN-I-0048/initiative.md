---
id: tier-3-architectural-drive-out
level: initiative
title: "Tier-3 architectural drive-out — config refactor, event bus, summarizer, learning, triage"
short_code: "ARAWN-I-0048"
created_at: 2026-05-15T20:56:49.394584+00:00
updated_at: 2026-05-15T20:56:49.394584+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: tier-3-architectural-drive-out
---


## Context **[REQUIRED]**

ARAWN-I-0044's tier 1 is complete (8 tasks shipped). Tier 2-late is captured on
that initiative for telemetry-triggered revisits. What remains is **tier 3** —
five open architectural questions surfaced by the openhuman comparative dive,
recorded in spec **ARAWN-S-0004** §A–E. Those questions are bigger than tasks:
each one is a directional decision about how arawn evolves over the next 1–2
quarters.

This initiative is the drive-out vehicle. The spec stays the working document
where each `Decide:` line records the outcome. This initiative tracks
progress on the spec, generates follow-on tasks when a question resolves
"adopt", and closes when every question has a recorded decision (including
"defer").

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- Resolve every tier-3 question in ARAWN-S-0004 §A–E with a recorded
  `Decide:` line: *adopt now*, *adopt after X*, *defer indefinitely*, or
  *reject with rationale*.
- For each *adopt* outcome, spin out concrete tasks under this initiative
  with the same discipline as tier 1 (acceptance criteria, tests,
  deviation logs).
- Keep the spec as the single source of truth for the *why* behind each
  decision so the openhuman comparative reasoning is preserved.

**Non-Goals:**
- Re-litigating tier 4. Those decisions (Composio backend proxy, channel
  sprawl, agentmemory shared backend, subconscious heartbeat) are locked
  in ARAWN-S-0004 §F–I.
- Touching the tier-1 / tier-2-late tracking — that lives on I-0044.

## Open questions (carry from ARAWN-S-0004)

| ID | Question | Status |
|---|---|---|
| §A | Per-domain typed `Config` refactor | open |
| §B | Typed cross-module event bus | open |
| §C | Tree summarizer placement (year→month→day→hour) | open |
| §D | Learning candidate/producer split | open |
| §E | Triage drop/notify/act tier | open |

Each row's full context lives in the spec. The status column above mirrors
the spec — update both when a decision lands.

## Detailed Design **[REQUIRED]**

### Working rhythm

1. Pick one question. Read its spec section. Walk through the tradeoff with
   the human-in-the-loop check that Metis requires for initiatives.
2. Land a recorded `Decide:` in the spec, including a one-paragraph
   rationale.
3. If the decision is *adopt now*, decompose into tasks under this
   initiative using the same shape as tier 1: acceptance criteria,
   reference paths in `/tmp/openhuman/...`, deviation log on landing.
4. If the decision is *defer* or *reject*, the row in the spec gets the
   rationale and this initiative tracks no follow-on work for it.

### Dependencies

None of the §A–E questions are hard blockers for in-flight work.

The ceremony initiatives (I-0041 daily, I-0042 weekly, I-0043 retro)
and the integration initiatives (I-0045 GitHub, I-0046 Linear,
I-0047 Google Docs comments) were all scoped *before* the openhuman
comparative dive and carry their own design intent. §C (tree
summarizer) and §E (triage drop tier) are decisions about whether
to adopt openhuman's patterns as a shared backend, not prerequisites
those other initiatives have to wait on. If §C / §E later resolve
to *adopt now*, the ceremony or integration work may opt to refactor
onto the shared layer; if they resolve to *defer / reject*, the
existing scoping stands.

The §A–E questions can be resolved in any order. Priority is
operator-driven (which decision is the user most curious about right
now), not blocked by downstream work.

## Alternatives Considered **[REQUIRED]**

- **Keep tier 3 inside I-0044.** Rejected: I-0044's tier-1 work is shipped
  and the initiative wants to be "completed" for tracking hygiene. Tier 3
  is a slow, deliberate set of decisions; mixing it into a completed
  initiative obscures both.
- **Spin one initiative per question.** Rejected: §A–E share a single
  spec, share a single working rhythm, and resolve at similar rates.
  Five micro-initiatives would create more bookkeeping than work tracked.
- **Defer tier 3 indefinitely.** Rejected: §C blocks the ceremony
  initiatives and §E blocks the integration initiatives — leaving these
  open would silently block downstream work.

## Implementation Plan **[REQUIRED]**

Discovery-phase deliverables:

1. Walk §A–E in whatever order makes sense to the operator. No hard
   ordering — each question is independent and the in-flight
   ceremony / integration initiatives are not gated on these
   decisions (see Dependencies above).
2. For each `Decide:`, follow Metis's human-in-the-loop discipline for
   initiative-level architectural decisions.
3. Track adopt-decisions as tasks on this initiative.

Rough sense of effort, smallest to largest, for picking what to do
when you have an afternoon:
- §E triage drop tier — smallest, mostly a layer in front of
  `workstream_router`.
- §D learning candidate/producer split — needs an ADR before code;
  ADR effort, no code today.
- §B typed event bus — moderate; pays off when ≥3 cross-cutting
  consumers exist.
- §C tree summarizer — moderate; standalone library, no consumer
  pressure.
- §A per-domain config refactor — largest; touches every subsystem.

## Exit Criteria

- Every tier-3 question in ARAWN-S-0004 has a recorded `Decide:` line.
- Each `Decide: adopt-now` outcome has matching tasks landed on this
  initiative.
- The spec moves to `published`.

## Related

- ARAWN-I-0044 — openhuman comparative adoption (tier 1 complete, tier
  2-late tracked).
- ARAWN-S-0004 — the spec that carries §A–E details and decisions.
- ARAWN-I-0041 / I-0042 / I-0043 — ceremony initiatives that depend on §C.
- ARAWN-I-0045 / I-0046 / I-0047 — integration initiatives that depend on §E.
