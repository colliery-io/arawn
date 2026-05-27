# Lens CLI

*Reference. Slug rules, lifecycle, metadata fields, `identity_profile`.*

Source: `crates/arawn-core/src/lens.rs`, `crates/arawn-storage/src/lens_store.rs`.

For the agent-side tools, see [lens tools reference](./lens-tools.md). For the curation walkthrough, see [curate a lens](../how-to/curate-a-lens.md).

## Slug validation

Lens names are slugs and validated. The rules (source: `validate_name` in `lens.rs`):

| Rule | Effect on invalid input |
|---|---|
| Non-empty | `LensNameError::Empty` |
| ≤64 characters | `TooLong` |
| First char is ASCII lowercase letter or digit | `BadLeading` |
| Rest is `[a-z0-9_-]` | `BadChar('<c>')` |

ASCII-only. No uppercase (so we don't end up with `work` + `Work` + `WORK` as separate lenses). No spaces.

Examples — valid: `work`, `home`, `arawn-dev`, `q3_2026`, `7-falcon`. Invalid: `Work` (capital), `home space` (space), `-leading-dash`, `way-too-long-name…` (over 64).

## Reserved slugs

- `scratch` — the default lens for one-off / ad-hoc sessions. Auto-created on first boot at `<data_dir>/lenses/scratch/`. Cannot be deleted. Sessions in scratch can be promoted to a named lens via `/promote <name>` or `lens_promote`.

## Lifecycle

### Create

```
/lens create <slug>
```

Calls `lens_new`. The agent walks through an ontology proposal flow — proposes 5-12 tags based on lens intent, confirms with you, persists. After confirmation, the lens exists at:

```
<data_dir>/lenses/<slug>/
  ├── memory.db        # lens knowledge base
  └── workspace/       # FS-isolated working directory for shell + file tools
```

`identity_profile` defaults to `assistant` on create.

### List

```
/lens list
```

Calls `lens_list`. Returns active lenses. To include archived: use `lens_list { include_archived: true }` directly.

### Switch

```
/lens switch <slug>
```

Sets the session's **write-target** lens — where new learnings file — until you switch again. It does **not** scope reads: `signal_*` / `memory_search` always search across every lens (pass `lens=` to narrow). Switching only redirects where future writes land.

### Show / bind / unbind / describe / delete (agent tools, not slash subcommands)

The TUI dispatcher accepts only `/lens create | list | switch`. The remaining lifecycle operations are agent tools — ask the agent in chat ("bind the `work` lens to feed `gmail-inbox-me`") and it calls the matching `lens_*` tool. The tools and their JSON shapes:

| Tool | What it does |
|---|---|
| `lens_show { name }` | Returns metadata: description, bindings, tag ontology, identity profile, root_dir, archived state. |
| `lens_bind { lens, uri }` | Bind a feed or GitHub URI (see schemes below). |
| `lens_unbind { lens, uri }` | Remove a binding. |
| `lens_describe { lens, ... }` | Update `description`, `display_name`, or `identity_profile`. |
| `lens_delete { lens }` | Soft-delete (sets `archived = true`). The data on disk is untouched. |

URI schemes accepted by `lens_bind`:

| Scheme | Meaning |
|---|---|
| `<feed_id>` | Direct feed binding (the feed must already exist). |
| `github:repo:owner/name` | Register a `github/repo-mirror` feed for that repo, then bind. |
| `github:org:owner` | List all repos in the org, register one `github/repo-mirror` per repo, bind all. Org binds **supersede** per-repo binds in the same lens. |

See [bind a lens to a feed](../how-to/bind-a-lens-to-a-feed.md).

### Promote

```
/promote <slug>
```

Calls `lens_promote`. Takes the current scratch session and moves it under the named lens. Session history, memory entries created in this session, and the session's feed bindings all rebase. Useful when an ad-hoc session turns into ongoing work.

## Metadata fields

The `Lens` struct (source: `crates/arawn-core/src/lens.rs:113`):

| Field | Type | Description |
|---|---|---|
| `id` | UUID | Stable id retained for session-linkage compatibility. The user-facing addressing primitive is `name`. |
| `name` | string | Slug (see validation rules above). Primary key in the registry. |
| `display_name` | string | Human label shown in `/lens list`. Defaults to `name`. |
| `description` | string | Free text fed into extractor prompts. |
| `root_dir` | path | On-disk root (`<data_dir>/lenses/<name>/`). |
| `bindings` | list&lt;string&gt; | Feed ids and/or URI schemes bound to this lens. |
| `archived` | bool | Soft-delete flag. |
| `identity_profile` | enum | `assistant` (default) or `coding`. Selects the system-prompt persona. |
| `created_at` / `updated_at` | RFC3339 | Timestamps. |

The **tag ontology** is NOT a struct field. It lives in a sibling per-lens table (`TagOntologyStore` in `crates/arawn-memory/src/ontology.rs`) opened from the lens's `memory.db`. Manage it via `lens_propose_ontology` (used by `lens_new`), `lens_tag { op: "add" | "remove" | "list" }`, or accept tag-promoter proposals via `lens_apply`. Tools that read it (`signal_query`, `lens_show`, the extractor) load the ontology from that table at query time.

## `identity_profile`

A per-lens attribute that selects which system-prompt persona arawn loads when a session is bound to this lens:

| Value | Persona |
|---|---|
| `assistant` (default) | "Personal agentic assistant — watch, check, summarize, nudge." Tuned for cross-tool reasoning, summarization, drafting, follow-up. |
| `coding` | Software engineering tool — bug fixes, refactoring, code explanation. Optional opt-in for engineering lenses. |

Source: `crates/arawn-engine/src/system_prompt.rs` (`ASSISTANT_*` vs `CODING_*` const sets). `LocalService::build_engine_config` picks per lens.

### Setting `identity_profile`

```text
lens_describe { "identity_profile": "coding" }
```

Or alter the SQLite row directly:

```sql
UPDATE lenses SET identity_profile = 'coding' WHERE name = 'arawn-dev';
```

Defaults to `assistant` on every new lens — even ones whose name suggests coding work. Opt in explicitly.

The full design rationale is in [identity-by-lens explanation](../explanation/identity-by-lens.md).

## On-disk layout

```
<data_dir>/lenses/<slug>/
  ├── memory.db        # lens-scoped knowledge base
  └── workspace/       # writable working directory (shell + file_write target)
```

The `workspace/` subdirectory is the shell sandbox's write root when this lens is active. Writes outside `workspace/` fail with `Permission denied`. See [shell sandbox reference](./shell-sandbox.md).

## Migrations

- V10 (`crates/arawn-storage/migrations/V10__lens_identity_profile.sql`) — added the `identity_profile` column. Pre-V10 lenses default to `assistant` on first read.

## Related

- [Lens tools reference](./lens-tools.md) — `signal_*`, `lens_*` agent tools.
- [Bind a lens to a feed how-to](../how-to/bind-a-lens-to-a-feed.md).
- [Lenses explanation](../explanation/lenses.md) — what they are, when to create one.
- [Identity-by-lens explanation](../explanation/identity-by-lens.md).
