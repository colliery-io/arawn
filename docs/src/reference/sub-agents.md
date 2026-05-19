# Sub-agents

*Reference. The `agent` tool, built-in agent types, the `task_*` background-task family, and the 3-level nesting cap.*

A **sub-agent** is a focused agent loop with its own system prompt, its own tool allowlist, and an isolated session. The primary agent spawns sub-agents to delegate well-scoped tasks; results return as tool outputs.

Source: `crates/arawn-engine/src/{agent_defs.rs, tools/agent.rs, background.rs, tools/{task_list,task_output,task_stop}.rs}`.

## The `agent` tool

```json
{
  "tool": "agent",
  "arguments": {
    "subagent_type": "Explore",
    "prompt": "Find every place we call `workflow_create` and summarize how it's used.",
    "llm": "hint:heavy",
    "run_in_background": false
  }
}
```

Fields:

| Field | Type | Description |
|---|---|---|
| `subagent_type` | string | Name of an agent definition (see below). |
| `prompt` | string | The user-message-equivalent the sub-agent starts with. |
| `llm` | string (opt) | Override the agent definition's LLM (named profile from `arawn.toml`, e.g. `default`, `hint:heavy`). |
| `run_in_background` | bool (opt) | When `true`, returns a task id and lets the parent continue. Resolve later via `task_output` / `task_stop`. |

## Nesting cap

Sub-agents can spawn sub-agents up to **3 levels deep**. This is hard-coded in `crates/arawn-engine/src/tools/agent.rs`. Beyond that, the `agent` tool returns an error.

## Built-in agent types

Source: `crates/arawn-engine/src/agent_defs.rs::built_in_agents`.

### `general-purpose`

Default fallback. Inherits the parent's full toolset and full system prompt. Useful when no specialized type fits.

### `Explore`

Investigation-flavored sub-agent. Implemented as a **disallow list**: inherits the parent's full toolset minus `agent`, `file_edit`, and `file_write`. So `shell` and integration write tools remain available; the system prompt is what discourages writes, not the tool gate. Use for: "find every X in the codebase", "summarize what this directory does", "research a question across docs and feeds".

### `Plan`

Planning-flavored sub-agent. Same disallow-list approach as `Explore` (subtracts `agent`, `file_edit`, `file_write` from parent's tools) plus a system prompt oriented toward emitting a plan rather than acting. Use for: "draft a plan for implementing X", "what's the right order to migrate Y?"

> **Note:** the disallow-list semantics means the system prompt is doing most of the read-only enforcement. If you need hard guarantees, set `permission_mode = "plan"` for the session (see [permission model](./permissions.md)).

## User-defined agent types

Custom agent definitions load from `<data_dir>/agents/`. Each is a markdown file with YAML frontmatter (the parser is permissive about hyphen-vs-camel but the canonical keys are camelCase):

```markdown
---
name: my-agent
description: When to use this agent
tools: ["file_read", "grep", "shell"]
disallowedTools: ["file_write"]
model: hint:medium
maxTurns: 15
---

# System prompt

The body of this file is the system prompt the sub-agent receives.
```

Frontmatter fields (source: `agent_defs.rs::AgentDefinition`):

| Field | Type | Description |
|---|---|---|
| `name` | string | Becomes the `subagent_type` value. |
| `description` | string | "When to use this agent" — surfaced in the `agent` tool's description. |
| `tools` | list&lt;string&gt; | Allowed tool names. Omit (or use `"*"`) to inherit parent's toolset. |
| `disallowedTools` | list&lt;string&gt; | Subtract from `tools`. (Parser key is camelCase.) |
| `model` | string | LLM profile override. Falls back to parent's model. |
| `maxTurns` | int | Per-invocation iteration cap. Defaults to `None` (no cap beyond the parent loop's `max_iterations`). |

Plugins can also ship agent definitions — see [plugins reference](./plugins.md).

## Background tasks

When `run_in_background: true`, the `agent` tool returns immediately with a task id. The agent loop continues; the background sub-agent runs in parallel. Two tools operate on background tasks once you know the id:

| Tool | Description |
|---|---|
| `task_output <id>` | Read incremental output from a running task (does not block). |
| `task_stop <id>` | Cancel a running task. |

Source: `crates/arawn-engine/src/background.rs`, `tools/{task_output,task_stop}.rs`.

> **Note:** `task_create` / `task_update` / `task_list` / `task_get` are **session-todo** tools (a separate per-session todo list — `crates/arawn-engine/src/tools/task_list.rs`), not background-task management. The engine currently has no enumerator that returns "all running background sub-agents in this session"; the caller is expected to track ids returned by `agent`.

### Task lifecycle states

| State | Meaning |
|---|---|
| `running` | Sub-agent is actively executing. |
| `completed` | Sub-agent returned. Output available via `task_output`. |
| `failed` | Sub-agent errored. Error message in the task record. |
| `stopped` | Cancelled via `task_stop`. |

## When to spawn a sub-agent

| Spawn a sub-agent when... | Don't, when... |
|---|---|
| The task is investigation-shaped (find, summarize, research) | The task needs the parent's full context |
| The parent's context would balloon with raw tool output | The result is small and direct |
| You want to parallelize multiple independent queries | The work is sequential and dependent |

The agent tool is most useful for protecting the parent's context window from large tool results (e.g. "grep the entire codebase for X" returns megabytes of matches; the sub-agent summarizes and returns a paragraph).

## Inspecting at runtime

```
/agents
```

Lists every loaded agent definition with its description, source (`built-in` / `user` / `plugin:<name>`), allowed tools, and model.

## Related

- [Plugins reference](./plugins.md) — plugins can declare agent types.
- [Skills reference](./skills.md) — similar concept; skills are stricter recipes, agents are more open-ended.
- [Agent tools reference](./agent-tools.md) — `agent` and `task_*` tools.
- [Slash commands reference](./slash-commands.md) — `/agents`.
