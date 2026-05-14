# Agent Read Patterns

Recipes for the agent's interaction with [workstream palaces](./index.md).
These are the tools the agent reaches for once you've created a
workstream, bound feeds, and the extractor has built up a KB.

The mental model:

```
READ:   signal_search   signal_query   signal_timeline
                        workstream_show

CURATE: workstream_dust (manual) — see steward.md for re-shelve / map / doorwatch / tag-promoter

REVIEW: workstream_journal  workstream_refine

COMMIT: workstream_apply   workstream_rollback

MANUAL: workstream_tag (list / add / remove)
```

All workstream-scoped tools default to the **active** workstream
(set via `/workstream switch <name>`) and accept an explicit
`workstream` argument to query a different one ad-hoc.

## Reading the KB

### `signal_search`

Free-text search over the active workstream's entities. FTS5 +
vector similarity (RRF-fused when an embedder is configured). Both
`tags_ontology` and `tags_discovered` participate in the FTS recall
side via the indexed content blob.

```text
signal_search { "query": "postgres", "limit": 10 }
```

Returns entities sorted by hybrid score with `tags_ontology` and
`tags_discovered` returned as separate fields so the agent can see
both vocabularies.

### `signal_query`

Structured filter. Use when you know what *shape* of entity you want.

```text
signal_query { "entity_type": "decision", "tags": ["postgres"], "since": "2026-03-01T00:00:00Z" }
```

- `entity_type`: `fact | decision | convention | preference | person | note`
- `tags`: any-of filter against `tags_ontology` by default.
- `include_discovered: true` widens the tag filter to also match
  `tags_discovered` — useful when you're searching for a concept
  that hasn't been promoted into the ontology yet.
- `since` / `until`: RFC3339 timestamps against `updated_at`.

Tag filters default to ontology because that's the reliable
substrate; the discovered set is noisy and meant for recall, not
deterministic filtering. The `include_discovered` flag lets you opt
in when you need both.

### `signal_timeline`

Chronological slice. Orders by `created_at` desc within an optional
`since`/`until` window. Each event is `{ts, kind: "entity_created",
entity}`.

```text
signal_timeline { "since": "2026-04-01T00:00:00Z", "limit": 25 }
```

Useful for "what's been happening in this workstream lately."

### `workstream_show`

Surfaces the active workstream's metadata — description, bindings,
and **the declared tag ontology**. Call this before a tag-filtered
search if you're not sure what tags exist:

```text
workstream_show { } → { name, display_name, description, bindings,
                        archived, tags_ontology: [...], ... }
```

## Curating the KB

### `workstream_dust` (manual)

Cluster + summarize cold entities. Manual trigger only — dust is
*not* in the auto-pass steward subroutine list.

```text
workstream_dust { "tags": ["falcon"], "min_cluster_size": 3,
                  "idle_days": 30, "limit": 5 }
```

When dust finds zero clusters, the response carries `available_tags`
(the full ontology) plus context-aware suggestions and a `hint`
field. The agent reads these and retries with a corrected filter
without you having to know the right magic strings.

```jsonc
// zero-cluster response shape:
{
  "workstream": "work",
  "clusters_found": 0,
  "proposals_written": 0,
  "available_tags": ["falcon", "ledger", "postgres", ...],
  "suggestions": [
    "retry without the `tags` filter to scan all ontology tags",
    "lower `min_cluster_size` (current default 3)"
  ],
  "hint": "no clusters formed — pick a tag from `available_tags`..."
}
```

Always-proposal: dust never auto-applies. The proposal lands as a
journal row; you commit via `workstream_apply`.

## Reviewing pending steward action

### `workstream_journal`

The full history. Every steward action (whether already-applied or
proposal-only) shows up here.

```text
workstream_journal { "limit": 20 } → { workstream, count, rows: [...] }
```

Each row carries `subroutine`, `action`, `applied`, `reverted_at`,
plus the parsed `inputs` and `outputs` payloads. Useful for "what
has the steward done this week?"

### `workstream_refine`

Filtered view: pending proposals only (`applied = false AND
reverted_at IS NULL`). This is what you call to see what the
steward wants you to look at.

```text
workstream_refine { "limit": 20 } → { workstream, count, proposals: [...] }
```

After tag-promoter has run a few times, this is where new ontology
suggestions surface. After map / door-watch passes, new relations
and identity candidates land here. After a manual dust trigger,
this is where the cluster summaries wait for approval.

## Committing or rejecting

### `workstream_apply <id>`

Commits a pending proposal. The apply path is per-subroutine:

- `dust/summarize` → inserts the summary entity + adds `SUMMARIZES`
  edges to its source entities.
- `map/propose_relation` → adds the proposed relation.
- `tag-promoter/promote_tag` → adds the tag to the workstream's
  ontology with `added_via=promotion`.
- `doorwatch/propose_identity` → metadata-only flip; the journal
  row's `applied=true` *is* the acceptance record (we don't have
  cross-workstream merge primitives yet).

```text
workstream_apply { "id": 42 } → { id: 42, status: "applied" | "already_applied" }
```

Idempotent. Refuses to apply a proposal that's already been rolled
back (returns an error).

### `workstream_rollback <id>`

Undoes a journal row. Per-subroutine inverse:

- `reshelve/merge` → restores both pre-state entities, removes the
  `SUPERSEDES` edge.
- `reshelve/delete` → re-inserts the deleted entity from the
  journaled snapshot.
- `dust/summarize` → deletes the summary entity (DETACH DELETE
  removes its `SUMMARIZES` edges too).
- `tag-promoter/promote_tag` → removes the tag from the ontology.
- `map / doorwatch` proposals → metadata flip only.

```text
workstream_rollback { "id": 42 } → { id: 42, status: "reverted" | "already_reverted" }
```

Idempotent. Repeated calls on a reverted row return
`already_reverted` rather than re-reverting.

## Manual ontology management

### `workstream_tag` (list / add / remove)

Direct CRUD on the workstream's tag ontology, outside the
propose/accept cycle. Use this when:

- You want to seed a tag the steward hasn't proposed yet.
- You want to retire a tag that's no longer useful.
- You want to see the full ontology with `added_via` provenance.

```text
workstream_tag { "op": "list" } → { workstream, count, tags: [...] }
workstream_tag { "op": "add", "tag": "calidor" } → { tag, status: "added" }
workstream_tag { "op": "remove", "tag": "calidor" } → { tag, status: "removed" | "not_found" }
```

For organic growth, prefer letting `tag-promoter` propose and
`workstream_apply` commit — that path leaves an audit trail in the
journal with `added_via=promotion` and a count + sample entity ids.
Manual `workstream_tag add` lands with `added_via=manual`.

## A typical session

```text
You> /workstream switch work
agent> [workstream_switch] → now in workstream 'work'

You> what did we decide about Postgres?
agent> [signal_search "postgres"] → "Postgres 16 for the ledger service",
       quoted with rationale.

You> are there any pending steward proposals I should look at?
agent> [workstream_refine] → 2 pending:
       - tag-promoter: promote `rfc-0042` (count: 4, sample ids: …)
       - tag-promoter: promote `cutover` (count: 5, sample ids: …)

You> Yes to rfc-0042, no to cutover.
agent> [workstream_apply id=17] → applied. `rfc-0042` is now in the ontology.
       [workstream_rollback id=18] → reverted. `cutover` is rejected.

You> Show me what we're tracking that's gone cold.
agent> [workstream_dust] → 1 cluster proposal:
       - 3 falcon entities, all >60 days idle. Summary: "Falcon project
         wrap-up: 99.7% successful cutover, 2-week-buffer lesson, code
         archived at github.com/acme/falcon-archived."

You> apply that summary.
agent> [workstream_apply id=19] → applied. Summary entity created.
```

## See also

- [Steward](./steward.md) — what each subroutine does and the journal
  contract.
- [Extraction](./extraction.md) — how the entities the agent reads
  got there.
- [Memory](../memory.md) — the global tier of the two-tier memory
  system; not workstream-scoped.
