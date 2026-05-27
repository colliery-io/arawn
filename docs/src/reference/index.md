# Reference

*Information-oriented. Austere lookup material. Structured for finding a specific answer, not reading end-to-end.*

Reference pages are dense and complete. They don't teach concepts — that's [Explanation](../explanation/index.md). They don't walk you through tasks — that's [How-to](../how-to/index.md). They tell you *exactly what exists* and what each thing does.

## Cross-cutting catalogs

- **[CLI flags & subcommands](./cli.md)** — every `arawn` subcommand and every flag.
- **[Slash commands](./slash-commands.md)** — every `/command` in the TUI, alphabetized.
- **[Configuration schema](./config-schema.md)** — every `[section]` and key in `arawn.toml`.
- **[Environment variables](./env-vars.md)** — every env var arawn reads.
- **[Data directory layout](./data-directory.md)** — full map of `~/.arawn/`.
- **[Troubleshooting](./troubleshooting.md)** — error → cause → fix tables for every common failure.

## Agent surface

- **[Agent tools](./agent-tools.md)** — every tool the agent can call.
- **[Permission model](./permissions.md)** — rule format, modes, audit log.
- **[Shell sandbox](./shell-sandbox.md)** — sandbox enforcement + safe-env allowlist.
- **[Integrations](./integrations.md)** — per-provider scopes, tools, permission-prompt rules.

## Lenses, ceremonies, todos

- **[Lens CLI](./lens-cli.md)** — `/lens {create,list,switch,bind,unbind,promote,delete}` + slug validation + `identity_profile`.
- **[Lens tools](./lens-tools.md)** — the 10 lifecycle + 3 signal tools the agent uses.
- **[Steward subroutines](./steward-subroutines.md)** — refine/apply/rollback machinery.
- **[Ceremonies tools](./ceremonies-tools.md)** — daily / weekly / retro tool family + cron defaults + retro detectors.
- **[Todos tools](./todos-tools.md)** — the 8 `todo_*` tools.

## Data model

- **[Feeds overview](./feeds-overview.md)** — on-disk layout, cadence, status.
- **[Feed templates](./feed-templates.md)** — all 17 templates, parameters, output shape.
- **[`feed_search` tool](./feed-search-tool.md)** — signature, ranking model, operational notes.
- **[Projection tables](./projection-tables.md)** — schemas + `ProjectionRow` struct.
- **[Palace types](./palace-types.md)** — entity + relation type catalog.
- **[Memory model](./memory-model.md)** — entity/relation/confidence tables + storage layout.
- **[Workflow tools](./workflow-tools.md)** — `workflow_create/list/status/delete` + cron + storage.

## Extensibility

- **[Plugins](./plugins.md)** — manifest, marketplaces, lifecycle, scopes.
- **[MCP](./mcp.md)** — `[[mcp.servers]]` config and stdio model.
- **[Skills](./skills.md)** — skill format + `/skill-name` invocation.
- **[Sub-agents](./sub-agents.md)** — `agent` tool + `task_*` family + agent_defs.

## When reference isn't what you need

- If you want to learn from zero, see **[Tutorials](../tutorials/index.md)**.
- If you want a recipe for a specific task, see **[How-to](../how-to/index.md)**.
- If you want to understand *why*, see **[Explanation](../explanation/index.md)**.
