# Lens and signal tools

*Reference. The agent tools that read from and curate a lens's palace.*

These tools default to the **active** lens (set via `/lens switch <name>`) and accept an explicit `lens` argument to query a different one ad-hoc.

For the CLI side (slash commands + slug rules), see [lens CLI reference](./lens-cli.md). For curation walkthroughs, see [curate a lens](../how-to/curate-a-lens.md).

Source: `crates/arawn-engine/src/tools/{signal,lens,steward}.rs`.

## Read tools

### `signal_search`

Free-text search over your curated entities **across all lenses** (each hit labeled with its source `lens`); pass a `lens` argument to restrict to one. FTS5 + vector similarity (RRF-fused when an embedder is configured). Both `tags_ontology` and `tags_discovered` participate in the FTS recall side via the indexed content blob.

```text
signal_search { "query": "postgres", "limit": 10 }
```

Returns entities sorted by hybrid score with `tags_ontology` and `tags_discovered` returned as separate fields so the agent can see both vocabularies.

### `signal_query`

Structured filter. Use when you know what *shape* of entity you want.

```text
signal_query { "entity_type": "decision", "tags": ["postgres"], "since": "2026-03-01T00:00:00Z" }
```

- `entity_type`: `fact | decision | convention | preference | person | note`
- `tags`: any-of filter against `tags_ontology` by default.
- `include_discovered: true` widens the tag filter to also match `tags_discovered` — useful for concepts that haven't been promoted into the ontology yet.
- `since` / `until`: RFC3339 timestamps against `updated_at`.

Tag filters default to ontology because that's the reliable substrate; the discovered set is noisy and meant for recall, not deterministic filtering. The `include_discovered` flag lets you opt in when you need both.

### `signal_timeline`

Chronological slice. Orders by `created_at` desc within an optional `since`/`until` window. Each event is `{ts, kind: "entity_created", entity}`.

```text
signal_timeline { "since": "2026-04-01T00:00:00Z", "limit": 25 }
```

Useful for "what's been happening in this lens lately."

### `lens_show`

Returns a lens's metadata — description, bindings, tag ontology, identity profile. Defaults to the session's write-target lens; pass a name for another.

```text
lens_show { } → { name, display_name, description, bindings,
                        archived, tags_ontology: [...], identity_profile, ... }
```

Useful before a tag-filtered search when you're not sure what tags exist.

## Lifecycle tools

Source: `crates/arawn-engine/src/tools/lens.rs`.

| Tool | Description |
|---|---|
| `lens_new` | Create a lens and walk through ontology proposal. |
| `lens_list` | List active (non-archived) lenses. |
| `lens_switch` | Set the write-target lens (where new learnings file; reads stay cross-lens). |
| `lens_describe` | Update description, display_name, or `identity_profile`. |
| `lens_bind` | Bind a feed by id, or `github:repo:owner/name` / `github:org:owner`. |
| `lens_unbind` | Remove a binding. |
| `lens_promote` | Promote scratch session to a named lens. |
| `lens_delete` | Soft-delete (sets `archived = true`). Reversible by re-adding. |

`lens_propose_ontology` is a helper used internally by the `lens_new` flow; the agent rarely calls it directly.

## Manual ontology management

### `lens_tag`

Direct CRUD on the lens's tag ontology, outside the propose/accept cycle. Use this when:

- You want to seed a tag the steward hasn't proposed yet.
- You want to retire a tag that's no longer useful.
- You want to see the full ontology with `added_via` provenance.

```text
lens_tag { "op": "list" } → { lens, count, tags: [...] }
lens_tag { "op": "add", "tag": "calidor" } → { tag, status: "added" }
lens_tag { "op": "remove", "tag": "calidor" } → { tag, status: "removed" | "not_found" }
```

For organic growth, prefer letting `tag-promoter` propose and `lens_apply` commit — that path leaves an audit trail in the journal with `added_via=promotion`. Manual `lens_tag add` lands with `added_via=manual`.

## Curation tools

These go with the steward — see [steward subroutines reference](./steward-subroutines.md) for what each subroutine does.

| Tool | Description |
|---|---|
| `lens_refine` | List pending steward proposals (`applied = false AND reverted_at IS NULL`). |
| `lens_apply <id>` | Commit a pending proposal. Idempotent. |
| `lens_rollback <id>` | Undo an applied (or pending) proposal. Idempotent. |
| `lens_dust` | Manual trigger for the dust summarizer (cluster + summarize cold entities). |
| `lens_journal` | Full audit log: every steward action ever taken on this lens. |

### `lens_dust` details

Two cluster modes:

- `cluster_by: "tag"` (default) — group entities by shared `tags_ontology` value.
- `cluster_by: "provenance"` — group by shared `EXTRACTED_FROM` target.

For a cluster to summarize, every entity must have `updated_at < now - idle_days` (default 30).

When dust finds zero clusters, it returns `available_tags`, context-aware `suggestions`, and a `hint`:

```jsonc
{
  "lens": "work",
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

Dust **never auto-applies** — the proposal lands as a journal row; you commit via `lens_apply`.

## Related

- [Lens CLI reference](./lens-cli.md) — slash commands and slug rules.
- [Steward subroutines reference](./steward-subroutines.md) — what each subroutine proposes.
- [Curate a lens how-to](../how-to/curate-a-lens.md) — refine/apply/rollback flow.
- [Identity-by-lens explanation](../explanation/identity-by-lens.md) — `identity_profile` design.
