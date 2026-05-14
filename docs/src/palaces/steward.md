# The Steward

The steward is the continuous-curation subsystem for
[workstream palaces](./index.md). While the
[extractor](./extraction.md) is reactive — it runs when new
projection rows arrive — the steward is proactive: every workstream
gets a periodic pass that re-reads its KB and proposes (or applies)
maintenance actions.

The steward has four subroutines, each tightly scoped:

| Subroutine | Mode | What it does | Bound |
|---|---|---|---|
| **re-shelve** | Mutating | Dedupe near-duplicate entities; merge content; mark erroneous ones for deletion | Up to N merges per pass |
| **map** | Proposal-only | Suggest new relations between entities | Up to K proposals per pass |
| **door-watch** | Proposal-only | Suggest cross-workstream identity matches (same person in two palaces, etc.) | Up to K proposals per pass |
| **tag-promoter** | Proposal-only | Promote a recurring `tags_discovered` value into the workstream's ontology | Up to K proposals per pass |
| **dust** | Manual trigger only | Summarize stale clusters of entities into a single Note | User-invoked via `workstream_dust` |

All four obey [ADR-0003](#)'s bounded blast-radius contract. Every
action is journaled write-ahead with enough payload to undo.

## The journal

Each workstream has its own append-only `steward_journal` table
colocated with its `memory.db`:

```sql
CREATE TABLE steward_journal (
    id INTEGER PRIMARY KEY,
    ts TEXT NOT NULL,
    subroutine TEXT NOT NULL,     -- 'reshelve' | 'map' | 'doorwatch' | 'tag-promoter' | 'dust'
    action TEXT NOT NULL,         -- 'merge' | 'delete' | 'propose_relation' | 'propose_identity' | 'promote_tag' | 'summarize'
    inputs_json TEXT NOT NULL,    -- what the subroutine considered
    outputs_json TEXT NOT NULL,   -- diff payload (sufficient for revert)
    model TEXT NOT NULL,
    prompt_hash TEXT NOT NULL,
    applied INTEGER NOT NULL,     -- 1 for mutating subroutines on action; 0 for proposals
    reverted_at TEXT              -- null until rolled back
);
```

Two flavors of row land in this table:

- **Already-applied** (`applied = 1`): re-shelve writes these the
  moment it acts. The row's `outputs_json` carries the pre-state
  needed to undo (pre-merge entity snapshots, the deleted entity's
  full record).
- **Proposal** (`applied = 0`): map / door-watch / tag-promoter
  write these when they want the user to confirm. dust also writes
  proposals — it never auto-applies. Acceptance happens via
  `workstream_apply <id>`; rejection via `workstream_rollback <id>`.

## Extract → Suggest → Add

The vocabulary-growth cycle for the tag ontology is the canonical
example of how all the pieces fit:

1. **Extract.** The extractor emits `tags_discovered` on every entity.
   These are free-form LLM tags outside the workstream's ontology.
2. **Suggest.** The `tag-promoter` subroutine runs every steward pass.
   It counts `tags_discovered` frequencies, dedupes against the
   ontology and against still-pending proposals, and writes a
   journal row `(tag-promoter, promote_tag, {tag, count, sample_entity_ids})`
   for every tag that crosses the threshold.
3. **Add.** The user reviews via
   [`workstream_refine`](./agent-read-patterns.md#workstream_refine).
   Accept via `workstream_apply <id>` — the apply path inserts the
   tag into `workstream_tag_ontology` with `added_via = 'promotion'`.
   Reject via `workstream_rollback <id>`.

After acceptance, the next extraction pass sees the new tag in the
ontology and the LLM can use it on subsequent entities.

The same propose-accept-rollback shape works for `map` proposals
(adds the relation on apply), `dust` proposals (inserts the summary
entity on apply, deletes it on rollback), and `door-watch` proposals
(metadata-only; the journal row *is* the acceptance record — we
don't have cross-workstream merge primitives yet).

## Blast radius caps

Each subroutine has a per-pass cap, configurable in `arawn.toml`.
Defaults are placeholders — real values come out of a Phase-5 tuning
harness once we have telemetry from real workstreams. When a pass
would exceed its cap, it writes a journal note and stops. Bounded
damage.

The `is_mutating()` flag on each subroutine gates writes through
`JournalGate` — a proposal-only subroutine that tries to write
`applied=true` rows gets rejected with a `StewardError::Subroutine`
before the row hits the table.

## Re-shelve specifics

The most consequential subroutine — it actually changes the graph.
Per ADR-0004 the allowed verbs are:

- `mark superseded` + add a `SUPERSEDES` edge to the survivor
- `set merged_into` pointer property on the deprecated entity
- `combine content fields` (copy non-empty fields from deprecated into
  survivor)
- `DELETE entity` — only when the LLM judges the entity *erroneous*,
  not merely duplicate

Forbidden: removing `EXTRACTED_FROM` provenance edges or the
projection rows they point at. Provenance is a one-way write.

Survivor selection is Rust-side, not LLM-side: the entity with the
higher `reinforcement_count` wins; ties break on newer `created_at`.
The LLM only judges *whether* to merge and proposes a
`combined_content` string; it doesn't choose the winner.

Trigger: only entities `updated_at > cursor` (created or touched since
the last re-shelve pass). A `steward_cursors` table colocates with the
journal in each workstream's KB. Monotonic advance via SQL CASE.

## Dust specifics

Dust is the only subroutine that's **manual-only** — it's not in the
auto-pass subroutine list. The agent invokes
[`workstream_dust`](./agent-read-patterns.md#workstream_dust) when
you ask it to.

Two cluster modes:

- `cluster_by: "tag"` (default) — group entities by shared
  `tags_ontology` value.
- `cluster_by: "provenance"` — group by shared `EXTRACTED_FROM`
  target (i.e. all entities extracted from the same projection row /
  Slack thread / Gmail message).

For a cluster to summarize, every entity must have `updated_at <
now - idle_days` (default 30). Recently-extracted content is
*not* dust material — the dust scope is "things you used to care
about that haven't moved in a while."

When dust finds nothing it returns the workstream's `available_tags`
plus context-aware suggestions (drop the tags filter; lower
min_cluster_size; lower idle_days). The hint mechanism is the
agent's self-recovery loop.

## Apply / rollback dispatch

The accept and rollback paths are symmetric. Per-subroutine inverses
live in `arawn_steward::accept::apply_forward` and
`arawn_steward::rollback::apply_inverse`:

| (subroutine, action) | Apply does | Rollback does |
|---|---|---|
| `dust/summarize` | Insert summary entity + SUMMARIZES edges | Delete summary entity (DETACH DELETE cleans edges) |
| `map/propose_relation` | Add the relation | Metadata flip only (no graph change) |
| `tag-promoter/promote_tag` | Add tag to ontology with `added_via=promotion` | Remove the tag from ontology |
| `doorwatch/propose_identity` | No graph change (flag flip is the record) | No graph change |
| `reshelve/merge` | Already applied at action time | Restore both pre-state entities + delete SUPERSEDES edge |
| `reshelve/delete` | Already applied at action time | Re-insert the deleted entity from the journaled snapshot |

`workstream_rollback` is idempotent: calling it twice on the same
journal row is harmless (second call returns `already_reverted`).

## Where the steward runs

A single tokio interval task (default 1 hour) iterates active
workstreams and runs each subroutine in the configured order.
Spawned at server start if the workstream router is wired.

Dust is **not** in this task — it's manual via `workstream_dust`.
The cadence is conservative because each subroutine costs LLM calls;
real production tuning is part of the Phase-5 harness work.

## See also

- [Agent read patterns](./agent-read-patterns.md) — the tools to
  see (`workstream_journal`, `workstream_refine`), commit
  (`workstream_apply`), reject (`workstream_rollback`), and curate
  (`workstream_tag`, `workstream_dust`).
- [Extraction](./extraction.md) — the writer that produces the
  entities the steward maintains.
- ADR-0003 (blast radius contract) and ADR-0004 (ontology cycle)
  document the design rationale.
