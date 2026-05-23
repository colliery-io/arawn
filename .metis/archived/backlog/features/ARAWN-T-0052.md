---
id: agent-type-system-loadable-agent
level: task
title: "Agent type system — loadable agent definitions from .arawn/agents/ with frontmatter"
short_code: "ARAWN-T-0052"
created_at: 2026-04-03T01:01:33.109917+00:00
updated_at: 2026-04-03T01:43:50.689484+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Agent type system — loadable agent definitions from .arawn/agents/ with frontmatter

## Objective

Allow users to define specialized agent types via markdown files in `.arawn/agents/`. Each definition controls the agent's system prompt, allowed tools, model, max turns, and other behavior. The `subagent_type` parameter on the AgentTool selects which definition to use.

### Type: Feature | Priority: P1

- **User Value**: Focused agents (code review, test writing, docs) with constrained tools and custom prompts. Matches Claude Code's agent definition system.
- **Effort Estimate**: L

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Agent definitions loaded from `.arawn/agents/*.md` with YAML frontmatter
- [ ] Frontmatter fields: `name`, `description`, `tools` (allow list), `disallowedTools`, `model`, `maxTurns`, `color`
- [ ] Markdown body becomes the agent's system prompt
- [ ] `subagent_type` param on AgentTool selects definition by name
- [ ] Unknown type falls back to general-purpose agent
- [ ] Built-in agents (Explore, Plan) with appropriate tool restrictions
- [ ] Agent definitions cached per session, reloaded on change

## Implementation Notes

- `AgentDefinition` struct with all frontmatter fields
- `load_agents_dir()` scans `.arawn/agents/` and parses markdown+frontmatter
- `AgentTool::execute` looks up definition, filters tool registry, overrides system prompt
- Built-in agents defined in code with `source: "built-in"`
- Reference: Claude Code's `loadAgentsDir.ts` and `builtInAgents.ts`
- Depends on: AgentTool (done), ToolRegistry filtering by name list