# Steward subroutines

*Reference. The five steward subroutines, the journal schema, and the apply/rollback dispatch table.*

The steward is the continuous-curation subsystem for [workstream palaces](../explanation/palaces.md). While the [extractor](../explanation/extraction.md) is reactive (runs on new projection rows), the steward is proactive: every workstream gets a periodic pass that re-reads its KB and proposes (or applies) maintenance actions.

For curation workflow, see [curate a workstream](../how-to/curate-a-workstream.md). For the design rationale, see [steward explanation](../explanation/steward.md).

Source: `crates/arawn-steward/src/`. ADRs: [ADR-0003](#) (blast radius), [ADR-0004](#) (ontology cycle).

## Subroutines

| Subroutine | Mode | What it does | Bound |
|---|---|---|---|
| `reshelve` | Mutating | Dedupe near-duplicate entities; merge content; mark erroneous ones for deletion | Up to N merges per pass |
| `map` | Proposal-only | Suggest new relations between entities | Up to K proposals per pass |
| `doorwatch` | Proposal-only | Suggest cross-workstream identity matches | Up to K proposals per pass |
| `tag-promoter` | Proposal-only | Promote a recurring `tags_discovered` value into the ontology | Up to K proposals per pass |
| `dust` | Manual trigger only | Summarize stale clusters of entities into a single Note | User-invoked via `workstream_dust` |

All four obey ADR-0003's bounded blast-radius contract. Every action is journaled write-ahead with enough payload to undo.

## The journal

Each workstream has its own append-only `steward_journal` table colocated with `memory.db`:

```sql
CREATE TABLE steward_journal (
    id INTEGER PRIMARY KEY,
    ts TEXT NOT NULL,
    subroutine TEXT NOT NULL,    -- 'reshelve' | 'map' | 'doorwatch' | 'tag-promoter' | 'dust'
    action TEXT NOT NULL,        -- 'merge' | 'delete' | 'propose_relation' | 'propose_identity' | 'promote_tag' | 'summarize'
    inputs_json TEXT NOT NULL,   -- what the subroutine considered
    outputs_json TEXT NOT NULL,  -- diff payload (sufficient for revert)
    model TEXT NOT NULL,
    prompt_hash TEXT NOT NULL,
    applied INTEGER NOT NULL,    -- 1 for mutating subroutines on action; 0 for proposals
    reverted_at TEXT             -- null until rolled back
);
```

Two flavors of row land here:

- **Already-applied** (`applied = 1`): `reshelve` writes these the moment it acts. The `outputs_json` carries the pre-state needed to undo (pre-merge entity snapshots, the deleted entity's full record).
- **Proposal** (`applied = 0`): `map`, `doorwatch`, `tag-promoter` write these when they want user confirmation. `dust` also writes proposals — it never auto-applies. Acceptance via `workstream_apply <id>`; rejection via `workstream_rollback <id>`.

## Apply / rollback dispatch

| (subroutine, action) | `workstream_apply` does | `workstream_rollback` does |
|---|---|---|
| `dust/summarize` | Insert summary entity + `SUMMARIZES` edges | Delete summary entity (DETACH DELETE cleans edges) |
| `map/propose_relation` | Add the relation | Metadata flip only (no graph change) |
| `tag-promoter/promote_tag` | Add tag to ontology with `added_via=promotion` | Remove the tag from ontology |
| `doorwatch/propose_identity` | No graph change (flag flip is the record — cross-workstream merge isn't implemented yet) | No graph change |
| `reshelve/merge` | Already applied at action time | Restore both pre-state entities + delete `SUPERSEDES` edge |
| `reshelve/delete` | Already applied at action time | Re-insert the deleted entity from the journaled snapshot |

`workstream_apply` and `workstream_rollback` are both idempotent — repeated calls return `already_applied` / `already_reverted` rather than re-doing the action.

## Re-shelve specifics

The most consequential subroutine — it actually changes the graph. Per ADR-0004 the allowed verbs are:

- `mark superseded` + add a `SUPERSEDES` edge to the survivor.
- `set merged_into` pointer property on the deprecated entity.
- `combine content fields` (copy non-empty fields from deprecated into survivor).
- `DELETE entity` — only when the LLM judges the entity *erroneous*, not merely duplicate.

**Forbidden:** removing `EXTRACTED_FROM` provenance edges or the projection rows they point at. Provenance is a one-way write.

**Survivor selection** is Rust-side, not LLM-side: the entity with higher `reinforcement_count` wins; ties break on newer `created_at`. The LLM only judges *whether* to merge and proposes a `combined_content` string.

**Trigger:** only entities `updated_at > cursor` (created or touched since the last re-shelve pass). A `steward_cursors` table colocates with the journal. Monotonic advance via SQL CASE.

## Dust specifics

`dust` is **manual-only** — not in the auto-pass list. The agent invokes [`workstream_dust`](./workstream-tools.md) when you ask it to.

Two cluster modes:

- `cluster_by: "tag"` (default) — group entities by shared `tags_ontology` value.
- `cluster_by: "provenance"` — group by shared `EXTRACTED_FROM` target.

For a cluster to summarize, every entity must have `updated_at < now - idle_days` (default 30). Recently-extracted content is not dust material — dust scope is "things you used to care about that haven't moved in a while."

## Blast-radius caps

Each subroutine has a per-pass cap, configurable in `arawn.toml`. Defaults are placeholders — real values come from a tuning harness with telemetry from real workstreams. When a pass would exceed its cap, it writes a journal note and stops. Bounded damage.

The `is_mutating()` flag on each subroutine gates writes through `JournalGate` — a proposal-only subroutine that tries to write `applied=true` rows gets rejected with `StewardError::Subroutine` before the row hits the table.

## Cadence

A single tokio interval task (default 1 hour) iterates active workstreams and runs each subroutine in the configured order. Spawned at server start if the workstream router is wired.

`dust` is **not** in this task — it's manual via `workstream_dust`. The cadence is conservative because each subroutine costs LLM calls; real production tuning is part of the Phase-5 harness work.

## Extract → Suggest → Add (the canonical example)

The vocabulary-growth cycle for the tag ontology is the canonical example of how all the pieces fit:

1. **Extract.** The extractor emits `tags_discovered` on every entity. Free-form LLM tags outside the workstream's ontology.
2. **Suggest.** `tag-promoter` runs every steward pass. Counts `tags_discovered` frequencies, dedupes against the ontology and against still-pending proposals, and writes a journal row `(tag-promoter, promote_tag, {tag, count, sample_entity_ids})` for every tag that crosses the threshold.
3. **Add.** The user reviews via `workstream_refine`. Accept via `workstream_apply <id>` (inserts the tag into `workstream_tag_ontology` with `added_via = 'promotion'`). Reject via `workstream_rollback <id>`.

After acceptance, the next extraction pass sees the new tag in the ontology and the LLM can use it on subsequent entities.

The same propose-accept-rollback shape works for `map` proposals (adds the relation on apply), `dust` proposals (inserts the summary entity on apply, deletes it on rollback), and `doorwatch` proposals (metadata-only flag flip).

## Related

- [Curate a workstream how-to](../how-to/curate-a-workstream.md) — the user-facing workflow.
- [Workstream tools reference](./workstream-tools.md) — `workstream_refine` / `workstream_apply` / `workstream_rollback` / `workstream_dust` / `workstream_journal` / `workstream_tag`.
- [Steward explanation](../explanation/steward.md) — bounded blast radius, why proposal-vs-apply.
- [Extraction explanation](../explanation/extraction.md) — what produces the entities the steward maintains.
