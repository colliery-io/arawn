---
id: phase-c-9-reference-extensibility
level: task
title: "Phase C-9: Reference — extensibility (plugins, MCP, skills, sub-agents)"
short_code: "ARAWN-T-0341"
created_at: 2026-05-19T01:39:46.364535+00:00
updated_at: 2026-05-19T02:48:33.671420+00:00
parent: ARAWN-I-0051
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0051
---

# Phase C-9: Reference — extensibility (plugins, MCP, skills, sub-agents)

## Parent Initiative

[[ARAWN-I-0051]]

## Objective

Reference pages for the four extensibility surfaces — plugins, MCP, skills, sub-agents — none of which are user-documented today (the entire system is condensed to one bullet in `intro.md`). Each page covers the user-facing model, the CLI/config touchpoints, and where things live on disk.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement  
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

## Acceptance Criteria

- [x] `reference/plugins.md` — manifest format + 3 marketplace source types + user/project scopes + components (tools/agents/skills/mcp/commands) + hot-reload + on-disk layout. ~125 lines.
- [x] `reference/mcp.md` — `[[mcp.servers]]` schema + stdio model + tool naming convention + plugin-declared servers + permission behavior + caveats. ~95 lines.
- [x] `reference/skills.md` — markdown+frontmatter format + 2 invocation paths (agent `skill` tool, user `/skill-name`) + 3 sources (built-in, user, plugin) + the 2 built-in skills. ~85 lines.
- [x] `reference/sub-agents.md` — `agent` tool + 3 built-in types (`general-purpose`, `Explore`, `Plan`) + user-defined agents in `<data_dir>/agents/` + 3-level nesting cap + `task_*` background-task family. ~120 lines.
- [x] `SUMMARY.md` updated with all four new pages.
- [x] `angreal docs build` clean.

## Status Updates

### 2026-05-18 — Completed (uncommitted)

Resolves the audit's "plugin/MCP/skill/sub-agent condensed to one bullet in intro.md" finding — each subsystem now has a dedicated reference page with the manifest schema, the user surface, and the on-disk locations.

Source: `crates/arawn-engine/src/{plugins,skills,agent_defs.rs,background.rs,tools/{agent,task_list,task_output,task_stop,skill}.rs}` plus `crates/arawn/src/plugin_cmd.rs` plus `crates/arawn-mcp/`.

Decisions worth noting:
- Plugin component types listed exhaustively (tools, agents, skills, MCP servers, commands) so future plugin authors know what shapes the manifest can declare.
- MCP page documents the `mcp__<name>__<tool>` naming convention because users will see those tool names in `/tools` output but the convention isn't obvious.
- Sub-agents page calls out the 3-level nesting cap as a hard constraint — users hitting it will hit a clear error from `tools/agent.rs`.
- Skills page distinguishes `user-invocable: true` (`/skill-name` slash) vs default (agent-only via `skill` tool).