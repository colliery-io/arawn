---
id: 001-steward-bounded-blast-radius-what
level: adr
title: "Steward bounded blast radius — what it can change, journal, rollback contract"
number: 1
short_code: "ARAWN-A-0003"
created_at: 2026-05-13T03:46:59.335499+00:00
updated_at: 2026-05-13T04:01:26.875592+00:00
decision_date: 
decision_maker: 
parent: 
archived: false

tags:
  - "#adr"
  - "#phase/decided"


exit_criteria_met: false
initiative_id: NULL
---

# ADR-1: Steward bounded blast radius — what it can change, journal, rollback contract

*This template includes sections for various types of architectural decisions. Delete sections that don't apply to your specific use case.*

## Context

Phase 5 of I-0040 introduces the **steward** — a continuously-running subsystem that re-reads each workstream KB and applies four maintenance subroutines (re-shelve / dust / map / door-watch). Two of those subroutines (re-shelve, dust) *mutate* the KB; the other two (map, door-watch) emit proposals.

A continuously-running LLM that rewrites its own KB is powerful and scary. Without explicit guardrails it can:

- merge entities that *look* similar but aren't (dedupe the wrong thing → silent data loss),
- summarize a stale cluster into a single node and destroy the originals (no recovery),
- run away on cost / token budget,
- act on stale projections (re-merging entities a user just edited).

I-0040 lists this as a top risk: "A continuously-running LLM rewriting its own KB is powerful and scary."

This ADR locks down what the steward is *allowed* to change, how every change is journaled, and how the user reverts an action that turned out to be wrong.

## Decision

**1. Closed allowlist of subroutine actions.** The steward has exactly four subroutines, each constrained to one verb on the KB:

| Subroutine | Verbs allowed | Verbs forbidden |
|---|---|---|
| **re-shelve** | `mark superseded`, `add SUPERSEDES relation`, `set merged_into pointer property`, `combine content fields` (copy non-empty fields from the deprecated entity into the survivor), `DELETE entity` (only when the LLM judges the entity erroneous, not merely duplicate) | removing `EXTRACTED_FROM` provenance edges or the projection rows they point at |
| **dust** | `INSERT new summary entity`, `add SUMMARIZES relations to source entities` | `mark sources superseded`, any deletion |
| **map** | (proposal-only — writes to `steward_proposals` table) | any mutation of nodes/edges in the KB graph |
| **door-watch** | (proposal-only) | any mutation |

**Never delete provenance.** No subroutine may remove an `EXTRACTED_FROM` edge or the row it points at.

**2. Per-pass blast-radius caps.** Each subroutine has a configurable per-pass cap on actions (merges / summaries / proposals). Defaults are intentionally *not* baked into this ADR — initial values are placeholders in `arawn.toml`; real values come out of a test-harness exercise that runs the steward against representative workstream fixtures and measures convergence vs. damage rate. The harness is part of Phase 5 deliverables.

If a pass would exceed its cap, the steward writes a journal note and stops the subroutine. The cap protects against an LLM that goes pathological — you lose at most a cap-bounded amount of damage.

**3. Append-only journal in each workstream's KB.** Schema:

```sql
CREATE TABLE steward_journal (
    id INTEGER PRIMARY KEY,
    ts TEXT NOT NULL,                   -- RFC3339
    subroutine TEXT NOT NULL,           -- 'reshelve' | 'dust' | 'map' | 'doorwatch'
    action TEXT NOT NULL,               -- 'merge' | 'summarize' | 'propose_relation' | 'propose_identity'
    inputs_json TEXT NOT NULL,          -- entity ids / projection ids touched
    outputs_json TEXT NOT NULL,         -- diff payload — what to do to undo
    model TEXT NOT NULL,
    prompt_hash TEXT NOT NULL,
    applied INTEGER NOT NULL DEFAULT 1, -- 0 for proposal-only
    reverted_at TEXT                    -- null until rollback
);
```

Every steward action — *including proposals* — gets exactly one journal row. The row is written *before* the mutation (write-ahead) so a crash mid-mutation leaves a recoverable trace.

**4. Rollback contract.** `Journal::revert(action_id)` reads `outputs_json`, applies the inverse, and sets `reverted_at`. Per subroutine:

- **re-shelve** revert: unset `superseded` on `old`, remove the SUPERSEDES edge, clear `merged_into`. When the action was `combine content`, restore the survivor's pre-merge field snapshot from `outputs_json`. When the action was `delete entity`, re-insert from the full entity snapshot stored in `outputs_json` (the journal carries the entire deleted row so revert is reconstitutable).
- **dust** revert: delete the summary entity + its SUMMARIZES edges (the source entities were untouched).
- **map / door-watch** revert: mark the proposal as rejected (it never mutated, so revert is a metadata flip).

Revert is always reversible (you can re-revert by rolling the action forward); the journal records both directions as separate rows.

**5. Write-ahead, per-pass transactions.** Each subroutine pass runs inside a single sqlite transaction. The journal row is written first; if the mutation fails, the transaction rolls back and the journal entry never appears (atomic). Cross-subroutine ordering: subroutines run sequentially within a pass to keep transactional reasoning simple; parallelism is per-workstream, not per-subroutine.

**6. Workstream isolation.** A steward pass operates on exactly one workstream's KB. Cross-workstream effects (door-watch) write proposals to the *source* workstream's journal; they never mutate the target workstream.

## Rationale

The verb allowlist + caps + journal is the minimum viable safety harness for an autonomous LLM that mutates user data. The alternatives:

- **No caps, no journal.** Rejected: a steward bug in a single pass could empty a workstream KB silently.
- **Append-only KB (never mark superseded).** Rejected: it conflicts with the search-before-create dedup semantics arawn-memory already uses; reinforcement / supersession is the existing contract.
- **Pause the steward by default and gate every action behind user approval.** Rejected for the *mutating* subroutines (re-shelve / dust): the steward's value is continuous curation; gating defeats the purpose. Accepted for map / door-watch which are proposal-only by design.
- **External durable queue for journaling (e.g. cloacina runs).** Rejected: cloacina already journals workflow runs; we'd be duplicating. The KB-local sqlite journal is the right resolution because rollback semantics are KB-shaped.

## Consequences

### Positive

- A user can always undo a steward action; data loss is bounded.
- Caps make pathological-LLM blast radius observable and small.
- Provenance walks are guaranteed: `EXTRACTED_FROM` survives every subroutine.

### Negative

- The journal adds storage overhead (~one row per steward action — small).
- Per-subroutine caps mean a *very* messy KB can't be cleaned up in one pass; convergence is over many passes.
- Revert is per-action only; bulk-revert ("undo everything the steward did this week") is not a primitive — it's repeated single reverts. Acceptable for v1; revisit if asked.

### Neutral

- The closed-allowlist + journal pattern is reusable for any future autonomous mutator (e.g. a hypothetical "feeds janitor" that deletes stale feeds). Worth keeping the journal table generic enough to host other actor names.

## Alternatives Analysis **[CONDITIONAL: Complex Decision]**

{Delete if there's only one obvious solution}

| Option | Pros | Cons | Risk Level | Implementation Cost |
|--------|------|------|------------|-------------------|
| {Option 1} | {Benefits} | {Drawbacks} | {Low/Medium/High} | {Estimate} |
| {Option 2} | {Benefits} | {Drawbacks} | {Low/Medium/High} | {Estimate} |
| {Option 3} | {Benefits} | {Drawbacks} | {Low/Medium/High} | {Estimate} |

## Rationale **[REQUIRED]**

{Why did we choose this option over alternatives?}

## Consequences **[REQUIRED]**

### Positive
- {Benefit 1}
- {Benefit 2}

### Negative
- {Cost or drawback 1}
- {Cost or drawback 2}

### Neutral
- {Neutral consequence 1}

## Review Schedule **[CONDITIONAL: Temporary Decision]**

{Delete if decision is permanent}

### Review Triggers
- {Condition that would trigger review 1}
- {Condition that would trigger review 2}

### Scheduled Review
- **Next Review Date**: {Date}
- **Review Criteria**: {What to evaluate}
- **Sunset Date**: {When this decision expires if not renewed}