---
title: "Slash commands"
---
*Reference. Every `/command` the TUI accepts, alphabetized.*

Source: `crates/arawn-tui/src/command.rs::register_builtins` (~line 72).

## Quick alphabetical index

`/agents` · `/autonomy` · `/brief` · `/clear` · `/connect` · `/copy` · `/disconnect` · `/export` · `/feeds` · `/forget` · `/help` · `/integrations` · `/lens` · `/mcp` · `/memory` · `/permissions` · `/plugins` · `/remember` · `/retro` · `/session` · `/skills` · `/today` · `/todo` · `/tools` · `/usage` · `/watch` · `/week` · `/workflows`

Plus any `/skill-name` registered by user-invocable skills (see [skills reference](../skills/)).

## Reference

### `/autonomy ask|edits|full|plan`

Set the permission posture at runtime. The four values map 1:1 to
`PermissionMode`:

- `ask` — ask before mutating actions (default).
- `edits` — auto-allow file writes; ask for shell.
- `full` — full autonomy; agent never asks.
- `plan` — read-only plan mode; side-effects are denied.

To pin a starting posture across restarts, set `[permissions]
autonomy` in `arawn.toml`. See [permissions reference](../permissions/)
and [lock down permissions how-to](../../how-to/lock-down-permissions/).

### `/agents`

List available agent types — built-ins (`general-purpose`, `Explore`, `Plan`) plus any loaded from `<data_dir>/agents/`. See [sub-agents reference](../sub-agents/).

### `/brief`

Show today's combined daily + weekly brief — the cached ceremony tablets rendered as one quick view. Unlike `/today`, which shows the daily tablet alone, `/brief` stitches both together for a one-shot read. Tablets are composed by the ceremony cron / boot-time backfill, not by `/brief` itself; if no tablet exists yet, `/brief` shows a placeholder pointing at `/today`. See [ceremonies tools reference](../ceremonies-tools/).

### `/clear`

Clear the chat history for the current session. Does NOT delete server-side session data — just empties the visible buffer.

### `/connect <service>`

Start the OAuth flow for an integration. Services: `gmail`, `google_calendar`, `google_drive`, `slack`, `slack@<alias>`, `atlassian`, `github`. Opens your browser to the provider's consent screen. `slack` adds a Slack workspace named after its team; `slack@<alias>` names it. See [connect Slack](../../how-to/connect-slack/#connect-another-workspace).

### `/disconnect <service>`

Drop stored credentials for an integration. The provider's permissions page is untouched — revoke there separately if needed. A Slack workspace is `slack@<alias>`, and its tools go with it; plain `slack` works only while one workspace is connected.

### `/feeds [pause <id>|resume <id>|rm <id> [--yes]|run <id>]`

List or manage continual data feeds.

- `/feeds` — list all running feeds.
- `/feeds pause <feed_id>` — pause a feed; cron stops; data stays.
- `/feeds resume <feed_id>` — resume a paused feed.
- `/feeds rm <feed_id>` — remove the feed. Asks for confirmation; pass `--yes` to skip.
- `/feeds run <feed_id>` — trigger an immediate run.

> **Note:** older docs may have referred to `/unwatch`. The actual command is `/feeds rm`.

### `/forget <entity>`

Remove an entity from the knowledge base. The entity is identified by short code or by exact name. See [memory model reference](../memory-model/).

### `/help`

Show the list of registered slash commands plus a short description for each.

### `/integrations`

List registered integrations and their connection state. Shows tool count per connected integration. Takes no subcommands — use `/connect <svc>` and `/disconnect <svc>` for state changes.

> **Note:** all extended lens lifecycle operations (bind, unbind, show, describe, delete) are agent tools, not slash subcommands — ask the agent to perform them.

### `/lens create <name> | list`

Manage lenses. The TUI dispatcher accepts two subcommands:

- `/lens create <name>` — create a lens (standing, memory-aware extractor); the agent walks through ontology proposal.
- `/lens list` — list active lenses.

Lenses are not switched into — chat reads signals across every lens, and memory is global. Other lifecycle operations (bind, unbind, show, describe, delete) are exposed as agent tools — ask the agent in chat (e.g. *"bind the `work` lens to feed `gmail-inbox-me`"*) and it calls `lens_bind`, `lens_unbind`, etc. See [lens tools reference](../lens-tools/) and [lens CLI reference](../lens-cli/).

### `/mcp`

List connected MCP servers. See [MCP reference](../mcp/).

### `/memory`

Show a summary of the global memory store — statements of fact and behavioral tuning the agent treats as always-known. (Signals extracted from feeds into lens KBs are surfaced separately via `signal_*`; they don't appear here.) See [memory model reference](../memory-model/).

### `/permissions`

Show the active permission rules and recent allow/deny/ask decisions for the session. See [permissions reference](../permissions/).

### `/plugins`

List loaded plugins. See [plugins reference](../plugins/).

### `/remember <fact>`

Store a fact in the knowledge base. The agent will retrieve it via `memory_search` when relevant.

### `/retro`

Show this week's retro ceremony tablet. See [ceremonies tools reference](../ceremonies-tools/).

### `/session new | list`

Manage sessions.

- `/session new` — start a fresh session within the current lens.
- `/session list` — list resumable sessions.

### `/skills`

List available skills (built-in + plugin-provided + user-defined). See [skills reference](../skills/).

### `/today`

Show today's daily ceremony tablet. See [ceremonies tools reference](../ceremonies-tools/).

### `/todo`

Open the todo list modal — create, mark done, archive. See [todos tools reference](../todos-tools/).

### `/tools`

List available agent tools. See [agent tools reference](../agent-tools/).

### `/watch [<template> <feed_id> [k=v ...]] | /watch list <template>`

Register a continual data feed.

- `/watch` (no args) — open the guided registration form: pick a template, fill
  its fields (types + defaults shown, required marked `*`, feed-id always asked,
  cadence under "advanced"), Enter to register.
- `/watch <template> <feed_id> [params]` — register directly from the command
  line. Quote values with spaces: `root="/Users/me/My Notes"`.
- `/watch list <template>` — show what's available for that template (channels for Slack, projects for Jira, etc.).

Supports `since=<rfc3339>` for backfill. See [create a feed how-to](../../how-to/create-a-feed/).

### `/week`

Show this week's weekly ceremony tablet. See [ceremonies tools reference](../ceremonies-tools/).

### `/workflows [list | status <name>]`

Show installed workflows and their execution status. Detailed inspection via the `workflow_status` agent tool. See [workflow tools reference](../workflow-tools/).
