---
id: 001-workstream-tag-ontologies-required
level: adr
title: "Workstream tag ontologies — required, with Extract→Suggest→Add cycle"
number: 1
short_code: "ARAWN-A-0004"
created_at: 2026-05-14T13:42:59.708004+00:00
updated_at: 2026-05-14T13:44:42.789518+00:00
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

# ADR-1: Workstream tag ontologies — required, with Extract→Suggest→Add cycle

*This template includes sections for various types of architectural decisions. Delete sections that don't apply to your specific use case.*

## Context

I-0040's Phase-4 extractor emits free-form `tags` on every entity. The original bet (recorded in the initiative spec as a non-goal: "no per-workstream ontologies") was that vocabulary would drift toward stability through observed-reuse pressure — i.e. the chain prompt would feed back observed tag patterns into future extractions and the model would converge.

UAT 2026-05-14 falsified that bet in two consecutive runs:

1. **Run 1 (no vocabulary hint):** Three entities about the same project tagged with three different variant strings (`falcon-project`, `falcon`, `infrastructure+falcon`). Dust's exact-tag-string clustering produced zero clusters; the downstream apply/rollback turns cascaded into no-op territory.
2. **Run 2 (with "reuse existing tags" hint in prompt):** The model overcorrected. All project-specific tags collapsed to generic ones (`infrastructure`, `eng-org`). Half the entities in the `dnd` workstream emitted *empty* tag arrays. Even worse for clustering.

The empirical conclusion: free-form LLM tag emission, with or without reuse hints, does not produce a clustering substrate good enough for dust / signal_query / cross-entity reasoning. Either direction of prompt pressure (encourage specificity → drift; encourage reuse → collapse) trades one pathology for another.

## Decision

**Tags become a two-field structure with one closed and one open dimension. Workstream creation requires the closed dimension up-front.**

### 1. Two-field tag model on `Entity`

`Entity.tags: Vec<String>` is replaced by:

- `tags_ontology: Vec<String>` — drawn exclusively from a per-workstream **declared tag list** (the workstream's ontology). Rust filters the extractor's emission against the current ontology and drops anything not in the list. This is the substrate dust / signal_query / cluster operations run on.
- `tags_discovered: Vec<String>` — free-form LLM emission. Carries content the ontology hasn't yet absorbed. Searched by `signal_search` (recall priority) and acts as raw material for the Suggest step below.

### 2. Per-workstream tag ontology, required at creation

Each workstream owns a `workstream_tag_ontology` table colocated with its `memory.db`:

```sql
CREATE TABLE workstream_tag_ontology (
    tag TEXT PRIMARY KEY,
    added_at TEXT NOT NULL,         -- RFC3339
    added_via TEXT NOT NULL          -- 'manual' | 'promotion'
);
```

Workstream creation cannot succeed without a non-empty ontology. The `workstream_create` tool requires `tags_ontology` as a positional argument. The agent's natural-language create flow (driven by the `workstream-create` skill) walks the user through producing an initial list: ask description → call `workstream_propose_ontology` → confirm with user → finalize.

### 3. Extract → Suggest → Add cycle

The Extract→Suggest→Add cycle is the convergence mechanism that replaces the (failed) "drift toward stability" mechanism from I-0040's original bet.

- **Extract.** Each extractor pass reads the workstream's current ontology + a sample of recent `tags_discovered` strings, includes both in the prompt. LLM emits `tags_ontology` (closed) and `tags_discovered` (open) per entity. Rust filters `tags_ontology` to in-list values; `tags_discovered` passes through verbatim.
- **Suggest.** A new steward subroutine (`tag-promoter`, proposal-only) counts `tags_discovered` frequencies across active entities. When a discovered tag exceeds threshold N (default 3 entities), the subroutine writes a journal proposal `(subroutine: "tag-promoter", action: "promote_tag", outputs: { tag, count, sample_entity_ids })`. Caps per pass like every other steward subroutine.
- **Add.** Existing `workstream_apply <id>` accepts the proposal. `accept::apply_forward` gains a `(tag-promoter, promote_tag)` arm that inserts the tag into `workstream_tag_ontology(tag, added_via: 'promotion')`. Future extractions immediately see the expanded list.

Escape hatches:

- `workstream_tag list | add <tag> | remove <tag>` for manual ontology edits without going through the suggest cycle.
- `workstream_apply` on a tag-promoter proposal is the formal accept; `workstream_rollback` on a previously-accepted promotion removes the tag from the ontology (with the journal payload restoring on revert-of-revert).

### 4. Reversal of an I-0040 non-goal

I-0040 spec explicitly lists "per-workstream ontologies as a separate type system" as a non-goal, on the reasoning that "for a personal assistant where user domains are unpredictable, the wrong ontology is worse than no ontology." This ADR reverses that on empirical grounds: no ontology produced unusable clustering. The two-field hybrid preserves the *closed entity-type* enum (Decision, Fact, etc. stay closed) while opening a *per-workstream tag namespace*. We are not introducing per-workstream entity types.

The initiative spec will be amended to reflect this; ADR-0004 is the canonical record of the bet shift.

## Alternatives Analysis

| Option | Pros | Cons | Risk Level | Implementation Cost |
|---|---|---|---|---|
| Status quo (free-form tags + better prompts) | No schema change | Falsified twice in UAT; failure modes asymmetric and hard to tune | High (proven) | Low |
| Pure ontology (one field, closed list) | Maximum cluster reliability | Out-of-vocabulary content gets dropped silently; no raw material to grow vocabulary | Medium | Medium |
| Hybrid: ontology + discovered *(chosen)* | Reliable cluster substrate + recall surface + organic vocabulary growth via Suggest/Add | Schema split; two prompt fields; more moving parts | Low–Medium | Medium |
| LLM-proposed ontology only (no manual add) | Fully automatic | First extraction has nothing to anchor to; cold-start drift | Medium | Medium |

## Rationale

The hybrid is the only option that simultaneously: (a) gives dust a deterministic substrate, (b) preserves LLM recall for novel concepts, (c) provides an organic growth path (Suggest→Add). The Add step is human-gated through the existing journal/refine/apply infrastructure we already shipped in T-0259/T-0260 — no new mutation primitives, just a new payload type.

The required-ontology-at-creation rule front-loads the user investment but only by a small amount: the agent's create flow proposes an initial list automatically from the workstream description. The user reviews/edits in one or two turns, not a 30-tag spreadsheet.

## Consequences

### Positive

- Dust and other clustering primitives operate on a reliable substrate; the UAT scenarios that previously cascaded on zero-cluster bugs become exercise paths again.
- The vocabulary growth mechanism is observable (journal entries) and reversible (rollback).
- `signal_search` retains LLM-recall via `tags_discovered`.

### Negative

- Schema break. Existing `Entity.tags` consumers must update. I-0040 explicitly accepts this since there's no userbase.
- Workstream creation gets a multi-turn agent flow instead of a single tool call — slightly longer onboarding.
- More LLM tokens per extraction (two prompt fields and one extra prompt section).

### Neutral

- Closed entity types (Decision, Fact, etc.) stay closed; we are not opening per-workstream type systems.
- Cross-workstream comparability via tags is not added here (each workstream's ontology is its own namespace). Door-watch identity proposals stay the cross-workstream surface.

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