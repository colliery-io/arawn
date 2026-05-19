# Slash commands

*Reference. Every `/command` the TUI accepts, alphabetized.*

Source: `crates/arawn-tui/src/command.rs::register_builtins` (~line 72).

## Quick alphabetical index

`/accept` · `/agents` · `/clear` · `/connect` · `/disconnect` · `/feeds` · `/forget` · `/help` · `/integrations` · `/mcp` · `/memory` · `/permissions` · `/plan` · `/plugins` · `/promote` · `/remember` · `/retro` · `/session` · `/skills` · `/today` · `/todo` · `/tools` · `/watch` · `/week` · `/workflows` · `/workstream`

Plus any `/skill-name` registered by user-invocable skills (see [skills reference](./skills.md)).

## Reference

### `/accept on|off|edits`

Set the permission mode at runtime. `on` (or `edits`) = `accept_edits` — file writes auto, shell asks. `off` = `default` — read-only auto, writes and shell ask. See [permissions reference](./permissions.md) and [lock down permissions how-to](../how-to/lock-down-permissions.md).

### `/agents`

List available agent types — built-ins (`general-purpose`, `Explore`, `Plan`) plus any loaded from `<data_dir>/agents/`. See [sub-agents reference](./sub-agents.md).

### `/clear`

Clear the chat history for the current session. Does NOT delete server-side session data — just empties the visible buffer.

### `/connect <service>`

Start the OAuth flow for an integration. Services: `gmail`, `google_calendar`, `google_drive`, `slack`, `atlassian`, `github`. Opens your browser to the provider's consent screen.

### `/disconnect <service>`

Drop stored credentials for an integration. The provider's permissions page is untouched — revoke there separately if needed.

### `/feeds [pause <id>|resume <id>|rm <id> [--yes]|run <id>]`

List or manage continual data feeds.

- `/feeds` — list all running feeds.
- `/feeds pause <feed_id>` — pause a feed; cron stops; data stays.
- `/feeds resume <feed_id>` — resume a paused feed.
- `/feeds rm <feed_id>` — remove the feed. Asks for confirmation; pass `--yes` to skip.
- `/feeds run <feed_id>` — trigger an immediate run.

> **Note:** older docs may have referred to `/unwatch`. The actual command is `/feeds rm`.

### `/forget <entity>`

Remove an entity from the knowledge base. The entity is identified by short code or by exact name. See [memory model reference](./memory-model.md).

### `/help`

Show the list of registered slash commands plus a short description for each.

### `/integrations`

List registered integrations and their connection state. Shows tool count per connected integration. Also supports `/integrations connect <svc>` and `/integrations disconnect <svc>` as aliases of `/connect` / `/disconnect`.

### `/mcp`

List connected MCP servers. See [MCP reference](./mcp.md).

### `/memory`

Show a summary of the knowledge base for the current scope (global + active workstream). See [memory model reference](./memory-model.md).

### `/permissions`

Show the active permission rules and recent allow/deny/ask decisions for the session. See [permissions reference](./permissions.md).

### `/plan`

Enter plan mode. The agent can read and think but every side-effect tool is denied (not asked) until you `/accept` out. See [permission model explanation](../explanation/permission-model.md).

### `/plugins`

List loaded plugins. See [plugins reference](./plugins.md).

### `/promote <name>`

Promote a scratch session to a named workstream. The session's history, memory, and feed bindings move under the new workstream.

### `/remember <fact>`

Store a fact in the knowledge base. The agent will retrieve it via `memory_search` when relevant.

### `/retro`

Show this week's retro ceremony tablet. See [ceremonies tools reference](./ceremonies-tools.md).

### `/session new | list`

Manage sessions.

- `/session new` — start a fresh session within the current workstream.
- `/session list` — list resumable sessions.

### `/skills`

List available skills (built-in + plugin-provided + user-defined). See [skills reference](./skills.md).

### `/today`

Show today's daily ceremony tablet. See [ceremonies tools reference](./ceremonies-tools.md).

### `/todo`

Open the todo list modal — create, mark done, archive. See [todos tools reference](./todos-tools.md).

### `/tools`

List available agent tools. See [agent tools reference](./agent-tools.md).

### `/watch <template> <feed_id> [k=v ...] | /watch list <template>`

Register a continual data feed.

- `/watch <template> <feed_id> [params]` — register the feed.
- `/watch list <template>` — show what's available for that template (channels for Slack, projects for Jira, etc.).

Supports `since=<rfc3339>` for backfill. See [create a feed how-to](../how-to/create-a-feed.md).

### `/week`

Show this week's weekly ceremony tablet. See [ceremonies tools reference](./ceremonies-tools.md).

### `/workflows [list | status <name>]`

Show installed workflows and their execution status. Detailed inspection via the `workflow_status` agent tool. See [workflow tools reference](./workflow-tools.md).

### `/workstream create <name> | list | switch <name> | bind <name> <uri> | unbind <name> <uri> | show <name>`

Manage workstreams.

- `/workstream create <name>` — create a workstream; agent walks through ontology proposal.
- `/workstream list` — list active workstreams.
- `/workstream switch <name>` — set the active workstream.
- `/workstream bind <name> <uri>` — bind a feed (by `feed_id`) or a GitHub URI (`github:repo:owner/name`, `github:org:owner`).
- `/workstream unbind <name> <uri>` — remove a binding.
- `/workstream show <name>` — show workstream metadata + bindings.

See [workstream CLI reference](./workstream-cli.md).
