---
title: "Reference"
---
*Information-oriented. Austere lookup material. Structured for finding a specific answer, not reading end-to-end.*

Reference pages are dense and complete. They don't teach concepts — that's [Explanation](../explanation/). They don't walk you through tasks — that's [How-to](../how-to/). They tell you *exactly what exists* and what each thing does.

## Cross-cutting catalogs

- **[CLI flags & subcommands](cli/)** — every `arawn` subcommand and every flag.
- **[Slash commands](slash-commands/)** — every `/command` in the TUI, alphabetized.
- **[Configuration schema](config-schema/)** — every `[section]` and key in `arawn.toml`.
- **[Environment variables](env-vars/)** — every env var arawn reads.
- **[Data directory layout](data-directory/)** — full map of `~/.arawn/`.
- **[Troubleshooting](troubleshooting/)** — error → cause → fix tables for every common failure.

## Agent surface

- **[Agent tools](agent-tools/)** — every tool the agent can call.
- **[Permission model](permissions/)** — rule format, modes, audit log.
- **[Shell sandbox](shell-sandbox/)** — sandbox enforcement + safe-env allowlist.
- **[Integrations](integrations/)** — per-provider scopes, tools, permission-prompt rules.

## Lenses, ceremonies, todos

- **[Lens CLI](lens-cli/)** — `/lens {create,list,switch,bind,unbind,promote,delete}` + slug validation + `identity_profile`.
- **[Lens tools](lens-tools/)** — the 10 lifecycle + 3 signal tools the agent uses.
- **[Steward subroutines](steward-subroutines/)** — refine/apply/rollback machinery.
- **[Ceremonies tools](ceremonies-tools/)** — daily / weekly / retro tool family + cron defaults + retro detectors.
- **[Todos tools](todos-tools/)** — the 8 `todo_*` tools.

## Data model

- **[Feeds overview](feeds-overview/)** — on-disk layout, cadence, status.
- **[Feed templates](feed-templates/)** — all 17 templates, parameters, output shape.
- **[`feed_search` tool](feed-search-tool/)** — signature, ranking model, operational notes.
- **[Projection tables](projection-tables/)** — schemas + `ProjectionRow` struct.
- **[Palace types](palace-types/)** — entity + relation type catalog.
- **[Memory model](memory-model/)** — entity/relation/confidence tables + storage layout.
- **[Workflow tools](workflow-tools/)** — `workflow_create/list/status/delete` + cron + storage.

## Extensibility

- **[Plugins](plugins/)** — manifest, marketplaces, lifecycle, scopes.
- **[MCP](mcp/)** — `[[mcp.servers]]` config and stdio model.
- **[Skills](skills/)** — skill format + `/skill-name` invocation.
- **[Sub-agents](sub-agents/)** — `agent` tool + `task_*` family + agent_defs.

## When reference isn't what you need

- If you want to learn from zero, see **[Tutorials](../tutorials/)**.
- If you want a recipe for a specific task, see **[How-to](../how-to/)**.
- If you want to understand *why*, see **[Explanation](../explanation/)**.
