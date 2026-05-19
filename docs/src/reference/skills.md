# Skills

*Reference. Skill format (markdown + YAML frontmatter) + the two invocation paths (user `/skill-name` and agent `skill` tool).*

A **skill** is a named, structured prompt — a tested recipe the agent can apply to a problem. Skills are stored as markdown files with YAML frontmatter. Two invocation paths: the user can type `/skill-name` in the TUI (when the skill is `user-invocable`), and the agent can call the `skill` tool with the skill name.

Source: `crates/arawn-engine/src/skills/`.

## File format

```markdown
---
name: my-skill
description: A short description shown in the autocomplete and the agent's tool list.
argument-hint: "<thing-to-do>"
allowed-tools: ["Read", "Glob", "Grep", "file_edit"]
model: hint:lightweight
user-invocable: true
---

# My skill

Detailed instructions for the agent. Reference the user's argument as `$ARGUMENTS`.

Step 1: ...
Step 2: ...
```

Frontmatter fields (parsed by `crates/arawn-engine/src/skills/definition.rs::parse_skill_markdown`):

| Field | Type | Default | Description |
|---|---|---|---|
| `name` | string | filename | Unique skill name. Lowercase + dashes recommended. |
| `description` | string | required | One-line summary shown in autocomplete and tool-list. |
| `argument-hint` | string | none | Hint shown in autocomplete (e.g., `"<filename>"`). |
| `allowed-tools` | list&lt;string&gt; | none | Restrict the agent's tool set while running this skill. None = inherit full toolset. |
| `model` | string | none | Override the LLM profile or hint shortcut (`hint:lightweight`, `hint:medium`, `hint:heavy`). |
| `user-invocable` | bool | `true` | When `true` (default), exposed as `/<name>` slash command in the TUI. Set to `false` to keep a skill agent-only. |

## Invocation paths

### Agent path — `skill` tool

The `skill` tool (source: `crates/arawn-engine/src/tools/skill.rs`) lets the agent invoke a named skill:

```json
{
  "tool": "skill",
  "arguments": { "skill": "workstream-create", "args": "work --description 'platform team'" }
}
```

The tool returns the skill's rendered prompt body (with `$ARGUMENTS` substituted) as the tool result. The parent agent then reads that text inline in the same loop and proceeds.

> **Note:** the `skill` tool does **not** spawn a sub-conversation today. The `allowed-tools` and `model` frontmatter fields are parsed into the `SkillDefinition` struct but are not enforced by the tool — the parent agent runs with its own permission rules and LLM. Treat `allowed-tools` as documentation-of-intent, not as a hard gate. If you need a real isolated context, use the `agent` tool (see [sub-agents reference](./sub-agents.md)).

### User path — `/skill-name` slash command

When `user-invocable: true`, the skill registers as a slash command. Typing `/skill-name <args>` in the TUI:

1. Sends a synthesized user message to the engine.
2. The engine runs the skill the same way the `skill` tool does.
3. The result streams back into the chat.

## Where skills come from

Three sources, all merged into the registry at startup:

1. **Built-in skills** — compiled into `crates/arawn-engine/src/skills/builtin/`. Currently: `workflows.md`, `workstream-create.md`. The engine reads these from include_str at compile time.
2. **User skills** — `<data_dir>/skills/` (loaded by `crates/arawn-engine/src/skills/loader.rs`).
3. **Plugin skills** — declared in a plugin manifest's `skills:` path. See [plugins reference](./plugins.md).

## Inspecting at runtime

```
/skills
```

Lists every loaded skill with its description, argument hint, source (`built-in` / `user` / `plugin:<name>`), and `user-invocable` flag.

## Argument substitution

Inside the skill body, `$ARGUMENTS` expands to whatever the user (or agent) passed. There's no positional argument parsing — the skill is responsible for any structured interpretation.

## Permission model

The `skill` tool itself is gated by the active permission mode like any other tool. The parent agent's actions *after reading the rendered skill body* are gated by its normal per-tool permission rules — `allowed-tools` in the skill frontmatter is documentation-of-intent, not enforcement.

## Examples

### `workflows` (built-in)

Recipe for composing a workflow from a conversational description. The user typically asks "build me a daily PR briefing"; the agent invokes the `workflows` skill, which produces the workflow JSON spec and calls `workflow_create`.

### `workstream-create` (built-in)

Recipe for creating a workstream — proposes an ontology, confirms with the user, writes the workstream. The `/workstream create` slash command routes through this skill.

## Related

- [Plugins reference](./plugins.md) — plugins can ship skills.
- [Sub-agents reference](./sub-agents.md) — similar concept; skills are stricter recipes, sub-agents are more open-ended.
- [Agent tools reference](./agent-tools.md) — the `skill` tool.
- [Slash commands reference](./slash-commands.md) — `/skills`.
