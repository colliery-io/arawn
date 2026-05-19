# The steward

*Explanation. Why bounded blast radius, why proposal-vs-apply, why the journal.*

The steward is the continuous-curation subsystem for [workstream palaces](./palaces.md). While the [extractor](./extraction.md) is reactive — it runs when new projection rows arrive — the steward is **proactive**: every workstream gets a periodic pass that re-reads its KB and proposes (or applies) maintenance actions.

For the subroutine list and apply/rollback dispatch, see [steward subroutines reference](../reference/steward-subroutines.md). For the user-facing workflow, see [curate a workstream](../how-to/curate-a-workstream.md). This page is about the design.

## The five subroutines, briefly

| Subroutine | Mode | What it does |
|---|---|---|
| `reshelve` | mutating | Dedupe near-duplicate entities; merge content; mark erroneous ones for deletion. |
| `map` | proposal-only | Suggest new relations between entities. |
| `doorwatch` | proposal-only | Suggest cross-workstream identity matches. |
| `tag-promoter` | proposal-only | Promote a recurring `tags_discovered` value to the ontology. |
| `dust` | manual-only | Summarize stale clusters into a single Note. |

Only `reshelve` mutates the graph without user confirmation. Everything else waits for `workstream_apply`.

## Why bounded blast radius

ADR-0003 captures the principle: **every steward action is journaled write-ahead with enough payload to undo, and is capped per pass.**

The risk of a non-bounded curator is correlated damage. If `reshelve` decides 200 entities are duplicates of each other and merges them all in one pass, a wrong call destroys 200 entities at once. The fix:

- **Per-pass caps.** Each subroutine has an `N` it can't exceed per pass. When the LLM proposes more than `N` actions, the pass writes a journal note and stops.
- **Write-ahead journaling.** Every mutating action's `outputs_json` carries the pre-state — pre-merge entity snapshots, the deleted entity's full record. Rollback is a deterministic replay.
- **Verb allowlist.** The Rust dispatcher in `arawn_steward::accept::apply_forward` and `arawn_steward::rollback::apply_inverse` knows exactly which verbs are legal per subroutine. The LLM doesn't get to invent new operations.

The result: a bad pass damages at most `N` entities, all recoverable.

## Why proposal-vs-apply

The four non-`reshelve` subroutines write `applied = 0` journal rows. They don't act; they propose. The user (or agent on the user's behalf) accepts via `workstream_apply`, which runs the forward path and flips `applied = 1`.

Why this two-step?

- **Stakes differ.** A tag-promoter proposal ("promote `cutover` to the ontology") is structurally safe — accepting it changes one row in a config table. A relation proposal ("`decision-42 supersedes question-19`") changes the graph's meaning. Both should pass through review.
- **Audit.** Every accept/reject is a journal row. `workstream_journal` lets you see every change ever made.
- **Reversibility.** Even if you accept and regret, `workstream_rollback <id>` is one call away. The forward path's inverse is per-subroutine, defined explicitly.

`reshelve` is the exception — it mutates immediately *because* its action is "merge near-duplicates," which is easy to reverse (restore both pre-state entities, delete the `SUPERSEDES` edge) and the friction of "did I want to merge these duplicates?" is too high to gate on a user. The pre-state in `outputs_json` is what makes rollback possible.

## Why per-workstream cursors

Each subroutine has a `steward_cursors` row that tracks "where did I get to last pass?" — monotonic by `updated_at`. The cursor advances via SQL CASE inside the same transaction that writes the journal row. Two consequences:

- **Idempotency.** A subroutine never re-processes an entity it already handled.
- **Crash safety.** If the process dies mid-pass, the cursor sits where the last completed action wrote it. The next pass resumes cleanly.

The cursor is per-(workstream, subroutine), not global — the steward can be partway through `reshelve` and have already finished `tag-promoter` for the same workstream.

## Why dust is manual

`dust` is the only subroutine that's user-triggered (`workstream_dust`). It clusters stale entities (`updated_at < now - idle_days`, default 30) and proposes summary Notes. Why not auto?

- **Compression is opinionated.** "Summarize these 5 entities into one Note" loses detail. The user should choose when that's the right move.
- **Cluster definitions need user input.** Cluster-by-tag vs cluster-by-provenance produces different summaries. The user picks.
- **It's a deliberate operation.** Most steward subroutines run silently in the background. Dust is meant to be a "Friday afternoon, let me clean this up" thing — a user-driven cleanup, not background entropy.

## The Extract → Suggest → Add cycle

The canonical example of how all the pieces fit, illustrated through the tag ontology:

```
1. Extract.  The extractor emits tags_discovered on every entity —
             free-form LLM tags outside the workstream's ontology.

2. Suggest.  tag-promoter runs every steward pass. Counts
             tags_discovered frequencies, dedupes against the
             ontology and against pending proposals, writes a
             journal row for every tag crossing the threshold.

3. Add.      The user reviews via workstream_refine. Accept via
             workstream_apply <id>; the apply path inserts the tag
             into the workstream's ontology with added_via=promotion.

After acceptance, the next extraction pass sees the new tag and the
LLM can use it on subsequent entities.
```

The same propose-accept-rollback shape works for `map`, `dust`, and `doorwatch`. It's the spine of the steward.

## Why cross-workstream identity is "metadata-only"

`doorwatch` proposes cross-workstream identity matches — "Alice in workstream A and Alice in workstream B might be the same person." Accepting writes the journal row but **doesn't change the graph**. Why?

We don't have cross-workstream merge primitives yet. The Right Thing — merge two `Person` entities across two databases — needs careful design (which workstream owns the survivor? what happens to per-workstream tags? what if the merge is wrong?). Today, accepting a `doorwatch` proposal is just a record that you agreed they're the same — useful for audit, no graph mutation. The mutation path lands in a later phase.

## Why the steward is bounded, period

The vision of arawn is "watch, check, summarize, nudge." Curation is part of *nudge* — the steward whispers, "have you noticed this duplicate?" or "want me to summarize this stale cluster?" It is not part of *act*.

Letting an LLM continuously rewrite your knowledge graph without bounds would inverse the relationship — your KB would become whatever the LLM thought it should be, not what you decided. The journal + verb allowlist + per-pass caps + propose-vs-apply discipline keeps the agent in its nudge role.

## Related

- [Steward subroutines reference](../reference/steward-subroutines.md) — the apply/rollback dispatch table + journal schema.
- [Curate a workstream how-to](../how-to/curate-a-workstream.md) — user workflow.
- [Palaces explanation](./palaces.md) — what the steward maintains.
- [Extraction explanation](./extraction.md) — what writes what the steward maintains.
