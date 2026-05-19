# Workstream CLI

*Reference. Slug rules, lifecycle, metadata fields, `identity_profile`.*

Source: `crates/arawn-core/src/workstream.rs`, `crates/arawn-storage/src/workstream_store.rs`.

For the agent-side tools, see [workstream tools reference](./workstream-tools.md). For the curation walkthrough, see [curate a workstream](../how-to/curate-a-workstream.md).

## Slug validation

Workstream names are slugs and validated. The rules (source: `validate_name` in `workstream.rs`):

| Rule | Effect on invalid input |
|---|---|
| Non-empty | `WorkstreamNameError::Empty` |
| ≤64 characters | `TooLong` |
| First char is ASCII lowercase letter or digit | `BadLeading` |
| Rest is `[a-z0-9_-]` | `BadChar('<c>')` |

ASCII-only. No uppercase (so we don't end up with `work` + `Work` + `WORK` as separate workstreams). No spaces.

Examples — valid: `work`, `home`, `arawn-dev`, `q3_2026`, `7-falcon`. Invalid: `Work` (capital), `home space` (space), `-leading-dash`, `way-too-long-name…` (over 64).

## Reserved slugs

- `scratch` — the default workstream for one-off / ad-hoc sessions. Auto-created on first boot at `<data_dir>/workstreams/scratch/`. Cannot be deleted. Sessions in scratch can be promoted to a named workstream via `/promote <name>` or `workstream_promote`.

## Lifecycle

### Create

```
/workstream create <slug>
```

Calls `workstream_new`. The agent walks through an ontology proposal flow — proposes 5-12 tags based on workstream intent, confirms with you, persists. After confirmation, the workstream exists at:

```
<data_dir>/workstreams/<slug>/
  ├── memory.db        # workstream knowledge base
  └── workspace/       # FS-isolated working directory for shell + file tools
```

`identity_profile` defaults to `assistant` on create.

### List

```
/workstream list
```

Calls `workstream_list`. Returns active workstreams. To include archived: use `workstream_list { include_archived: true }` directly.

### Switch

```
/workstream switch <slug>
```

Sets the active workstream. Subsequent sessions, feeds bound to the workstream, and signal queries all scope to this workstream until you switch again.

### Show / bind / unbind / describe / delete (agent tools, not slash subcommands)

The TUI dispatcher accepts only `/workstream create | list | switch`. The remaining lifecycle operations are agent tools — ask the agent in chat ("bind the `work` workstream to feed `gmail-inbox-me`") and it calls the matching `workstream_*` tool. The tools and their JSON shapes:

| Tool | What it does |
|---|---|
| `workstream_show { name }` | Returns metadata: description, bindings, tag ontology, identity profile, root_dir, archived state. |
| `workstream_bind { workstream, uri }` | Bind a feed or GitHub URI (see schemes below). |
| `workstream_unbind { workstream, uri }` | Remove a binding. |
| `workstream_describe { workstream, ... }` | Update `description`, `display_name`, or `identity_profile`. |
| `workstream_delete { workstream }` | Soft-delete (sets `archived = true`). The data on disk is untouched. |

URI schemes accepted by `workstream_bind`:

| Scheme | Meaning |
|---|---|
| `<feed_id>` | Direct feed binding (the feed must already exist). |
| `github:repo:owner/name` | Register a `github/repo-mirror` feed for that repo, then bind. |
| `github:org:owner` | List all repos in the org, register one `github/repo-mirror` per repo, bind all. Org binds **supersede** per-repo binds in the same workstream. |

See [bind a workstream to a feed](../how-to/bind-a-workstream-to-a-feed.md).

### Promote

```
/promote <slug>
```

Calls `workstream_promote`. Takes the current scratch session and moves it under the named workstream. Session history, memory entries created in this session, and the session's feed bindings all rebase. Useful when an ad-hoc session turns into ongoing work.

## Metadata fields

The `Workstream` struct (source: `crates/arawn-core/src/workstream.rs:113`):

| Field | Type | Description |
|---|---|---|
| `id` | UUID | Stable id retained for session-linkage compatibility. The user-facing addressing primitive is `name`. |
| `name` | string | Slug (see validation rules above). Primary key in the registry. |
| `display_name` | string | Human label shown in `/workstream list`. Defaults to `name`. |
| `description` | string | Free text fed into extractor prompts. |
| `root_dir` | path | On-disk root (`<data_dir>/workstreams/<name>/`). |
| `bindings` | list&lt;string&gt; | Feed ids and/or URI schemes bound to this workstream. |
| `archived` | bool | Soft-delete flag. |
| `identity_profile` | enum | `assistant` (default) or `coding`. Selects the system-prompt persona. |
| `created_at` / `updated_at` | RFC3339 | Timestamps. |

The **tag ontology** is NOT a struct field. It lives in a sibling per-workstream table (`TagOntologyStore` in `crates/arawn-memory/src/ontology.rs`) opened from the workstream's `memory.db`. Manage it via `workstream_propose_ontology` (used by `workstream_new`), `workstream_tag { op: "add" | "remove" | "list" }`, or accept tag-promoter proposals via `workstream_apply`. Tools that read it (`signal_query`, `workstream_show`, the extractor) load the ontology from that table at query time.

## `identity_profile`

A per-workstream attribute that selects which system-prompt persona arawn loads when a session is bound to this workstream:

| Value | Persona |
|---|---|
| `assistant` (default) | "Personal agentic assistant — watch, check, summarize, nudge." Tuned for cross-tool reasoning, summarization, drafting, follow-up. |
| `coding` | Software engineering tool — bug fixes, refactoring, code explanation. Optional opt-in for engineering workstreams. |

Source: `crates/arawn-engine/src/system_prompt.rs` (`ASSISTANT_*` vs `CODING_*` const sets). `LocalService::build_engine_config` picks per workstream.

### Setting `identity_profile`

```text
workstream_describe { "identity_profile": "coding" }
```

Or alter the SQLite row directly:

```sql
UPDATE workstreams SET identity_profile = 'coding' WHERE name = 'arawn-dev';
```

Defaults to `assistant` on every new workstream — even ones whose name suggests coding work. Opt in explicitly.

The full design rationale is in [identity-by-workstream explanation](../explanation/identity-by-workstream.md).

## On-disk layout

```
<data_dir>/workstreams/<slug>/
  ├── memory.db        # workstream-scoped knowledge base
  └── workspace/       # writable working directory (shell + file_write target)
```

The `workspace/` subdirectory is the shell sandbox's write root when this workstream is active. Writes outside `workspace/` fail with `Permission denied`. See [shell sandbox reference](./shell-sandbox.md).

## Migrations

- V10 (`crates/arawn-storage/migrations/V10__workstream_identity_profile.sql`) — added the `identity_profile` column. Pre-V10 workstreams default to `assistant` on first read.

## Related

- [Workstream tools reference](./workstream-tools.md) — `signal_*`, `workstream_*` agent tools.
- [Bind a workstream to a feed how-to](../how-to/bind-a-workstream-to-a-feed.md).
- [Workstreams explanation](../explanation/workstreams.md) — what they are, when to create one.
- [Identity-by-workstream explanation](../explanation/identity-by-workstream.md).
