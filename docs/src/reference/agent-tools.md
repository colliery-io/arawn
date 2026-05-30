# Agent tools

*Reference. Every tool the agent can call, grouped by category.*

Tools come from three sources: built-ins compiled into `arawn-engine`, integration tools registered when an integration connects, and plugin/MCP tools loaded at runtime. This page covers the built-ins and the integration tools. Plugin tools depend on which plugins you've installed — list them with `/plugins`. MCP tools depend on `[[mcp.servers]]` — list with `/mcp`.

Source: `crates/arawn-engine/src/tools/` plus `crates/arawn-integrations/src/<svc>/tools.rs`.

## Permission categories

Every tool is classified into one of four categories that the permission model uses:

| Category | Default behaviour | Examples |
|---|---|---|
| `ReadOnly` | auto-allow in every mode except `plan` | `Read`, `Glob`, `Grep`, `memory_search`, `signal_search` |
| `FileWrite` | ask in `ask`, auto in `edits`/`full`, deny in `plan` | `file_write`, `file_edit` |
| `Shell` | ask in `ask`/`edits`, auto in `full`, deny in `plan` | `shell` |
| `Other` | falls through to mode default | `web_fetch`, `web_search`, integration writes |

See [permissions reference](./permissions.md) for evaluation order.

## File and shell

| Tool | Category | Source | Description |
|---|---|---|---|
| `file_read` | ReadOnly | `file_read.rs` | Read a file. |
| `file_write` | FileWrite | `file_write.rs` | Write or overwrite a file. |
| `file_edit` | FileWrite | `file_edit.rs` | Exact-string search-and-replace in a file. |
| `glob` | ReadOnly | `glob.rs` | Filename glob match. |
| `grep` | ReadOnly | `grep.rs` | Content search via `rg`. |
| `shell` | Shell | `shell.rs` | Run a command in the OS sandbox. |

> **Note:** the tool names are lowercase (`file_read`, `glob`, `grep`). Capitalized names like `Read` / `Glob` / `Grep` appear in some legacy permission-rule examples but are not registered tool names and will not resolve at runtime.

## Web

| Tool | Category | Source | Description |
|---|---|---|---|
| `web_fetch` | Other | `web_fetch.rs` | Fetch a URL and return its content (HTML stripped to markdown). |
| `web_search` | ReadOnly | `web_search.rs` | Search the web. |

## Agent loop helpers

| Tool | Category | Source | Description |
|---|---|---|---|
| `think` | ReadOnly | `think.rs` | Scratchpad for chain-of-thought. No side effects. |
| `ask_user` | Other | `ask_user.rs` | Ask the user a question interactively. |
| `sleep` | Other | `sleep.rs` | Pause for N seconds (rare; used by polling flows). |
| `enter_plan_mode` | Other | `enter_plan_mode.rs` | Enter plan mode (every side-effect tool denied). |
| `exit_plan_mode` | Other | `exit_plan_mode.rs` | Leave plan mode. |

## Sub-agents

| Tool | Source | Description |
|---|---|---|
| `agent` | `agent.rs` | Launch a sub-agent (by `subagent_type`) with an isolated context. Built-in types: `general-purpose`, `Explore`, `Plan`. Custom types load from `<data_dir>/agents/`. 3-level nesting cap. |
| `task_list` | `task_list.rs` | Enumerate background sub-agent tasks in the current session — id, description, status, elapsed seconds. |
| `task_get` | `task_list.rs` | Point-in-time snapshot of a single background task by id (status + buffered output, never blocks). |
| `task_output` | `task_output.rs` | Read output from a background sub-agent task; blocks/polls until completion by default. |
| `task_stop` | `task_stop.rs` | Cancel a background sub-agent task. |

See [sub-agents reference](./sub-agents.md).

## Memory

| Tool | Source | Description |
|---|---|---|
| `memory_store` | `memory_store.rs` | Store a fact in the active scope's knowledge base. |
| `memory_search` | `memory_search.rs` | Hybrid FTS+vector search over the knowledge base. |

See [memory model reference](./memory-model.md).

## Signal (palace query)

Read across **every lens's** signal palace; each hit is labeled with its source lens. Pass `lens` to narrow to one.

| Tool | Source | Description |
|---|---|---|
| `signal_search` | `signal.rs` | Hybrid search across extracted entities, cross-lens. |
| `signal_query` | `signal.rs` | Filter by entity_type / tags / time window, cross-lens. |
| `signal_timeline` | `signal.rs` | Chronological view of recent entities, cross-lens. |

## Feed search

| Tool | Source | Description |
|---|---|---|
| `feed_search` | `feed_search.rs` | RRF-fused FTS + vector search across all running feeds. Independent of palace state. |

See [feed_search tool reference](./feed-search-tool.md).

## Lens lifecycle

Source: `lens/` (split into `create.rs`, `list.rs`, `show.rs`, `bind.rs`, `unbind.rs`, `describe.rs`, `delete.rs`, `propose_ontology.rs`).

| Tool | Description |
|---|---|
| `lens_new` | Create a lens + ontology. |
| `lens_list` | List active lenses. |
| `lens_show` | Show lens details + bindings. |
| `lens_describe` | Update description / display_name. |
| `lens_bind` | Bind a feed (direct or `github:repo:` / `github:org:` URI). |
| `lens_unbind` | Remove a binding. |
| `lens_delete` | Soft-delete (archives). |
| `lens_propose_ontology` | Helper used by the create flow. |

See [lens tools reference](./lens-tools.md) and [lens CLI reference](./lens-cli.md).

## Steward

Source: `steward.rs`.

| Tool | Description |
|---|---|
| `lens_refine` | List pending steward proposals. |
| `lens_apply` | Apply a proposal by journal id. |
| `lens_rollback` | Rollback an applied proposal. |
| `lens_dust` | Trigger the dust-summarizer subroutine. |
| `lens_journal` | View change history. |
| `lens_tag` | Tag management (promote / merge / split). |

See [steward subroutines reference](./steward-subroutines.md).

## Ceremonies

### Daily ceremony

Source: `daily.rs`. Default cron: `0 7 * * MON-FRI`.

| Tool | Description |
|---|---|
| `daily_run` | Run the daily ceremony now. |
| `daily_current` | Show today's tablet (also via `/today`). |
| `daily_list_items` | List items on the active tablet. |
| `daily_patch_item` | Edit / mark done / dismiss an item. |
| `daily_add_todo` | Add a todo into the tablet. |

### Weekly ceremony

Source: `weekly.rs`. Default cron: `0 7 * * MON`.

| Tool | Description |
|---|---|
| `weekly_run` | Run the weekly ceremony now. |
| `weekly_current` | Show this week's tablet (also via `/week`). |
| `weekly_list_items` | List items. |
| `weekly_list_priorities` | List proposed priorities. |
| `weekly_confirm_priority` | Confirm a priority. |
| `weekly_reject_priority` | Reject a priority. |
| `weekly_add_priority` | Add a priority manually. |

### Retro ceremony

Source: `ceremony.rs`. Default cron: Friday 16:00 local.

| Tool | Description |
|---|---|
| `retro_run` | Run retro now. |
| `retro_current` | Show this week's retro (also via `/retro`). |
| `retro_list_items` | List retro items. |
| `retro_save_diary` | Save the user's diary entry. |
| `retro_patch_item` | Edit / mark / dismiss. |

See [ceremonies tools reference](./ceremonies-tools.md).

## Todos

Source: `todo.rs`. Generic todo surface (I-0049).

| Tool | Description |
|---|---|
| `todo_create` | Create a todo. |
| `todo_list` | List todos. |
| `todo_get` | Get one todo by short code. |
| `todo_done` | Mark done. |
| `todo_undo` | Un-mark done. |
| `todo_patch` | Edit fields. |
| `todo_archive` | Archive. |
| `todo_search` | Search across todos. |

See [todos tools reference](./todos-tools.md).

## Workflows

Source: `crates/arawn-workflow/src/tools.rs`.

| Tool | Description |
|---|---|
| `workflow_create` | Author + compile + install a workflow from a JSON spec. |
| `workflow_list` | List installed workflows. |
| `workflow_status` | Show recent runs, success/failure counts. |
| `workflow_delete` | Uninstall. |

See [workflow tools reference](./workflow-tools.md).

## Skills

| Tool | Source | Description |
|---|---|---|
| `skill` | `skill.rs` | Invoke a named skill by id. Built-in skills: `workflows`, `lens-create`. |

See [skills reference](./skills.md).

## Integration tools

Available only when the corresponding integration is connected. Resolve via `/integrations`.

### Gmail (5)

`gmail_inbox_read`, `gmail_search`, `gmail_get_message`, `gmail_send`, `gmail_mark_read`.

### Google Calendar (3)

`calendar_upcoming`, `calendar_create_event`, `calendar_find_conflicts`.

### Google Drive (7)

`drive_search`, `drive_list`, `drive_get_metadata`, `drive_read`, `drive_upload`, `drive_update`, `drive_delete`.

### Slack (6)

`slack_list_channels`, `slack_history`, `slack_post`, `slack_react`, `slack_users_list`, `slack_open_dm`.

> **Note:** older docs used `slack_channels_list`, `slack_channel_history`, `slack_post_message`, and `slack_search`. The first three were renamed; `slack_search` is deferred and not in the registry.

### Atlassian — Jira (6)

`jira_search`, `jira_get_issue`, `jira_create_issue`, `jira_update_issue`, `jira_add_comment`, `jira_transition_issue`.

> **Note:** older docs used `jira_search_issues`. The actual name is `jira_search`.

### Atlassian — Confluence (5)

`confluence_search`, `confluence_get_page`, `confluence_create_page`, `confluence_update_page`, `confluence_list_spaces`.

### GitHub (0 — feeds only)

GitHub is read-only via the four feed templates (`github/notifications`, `github/issues-and-prs`, `github/review-queue`, `github/repo-mirror`). No agent tools today; v2 may add issue creation / PR comments.

See [integrations reference](./integrations.md) for per-provider OAuth + permission-prompt details.
