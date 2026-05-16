---
id: agent-tool-family-todo-and
level: task
title: "Agent tool family — todo_* and ceremony tool migration"
short_code: "ARAWN-T-0313"
created_at: 2026-05-16T22:52:34.501946+00:00
updated_at: 2026-05-16T22:52:34.501946+00:00
parent: ARAWN-I-0049
blocked_by: [ARAWN-T-0310]
effort: M
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0049
---

# Agent tool family — todo_* and ceremony tool migration

## Parent Initiative

[[ARAWN-I-0049]]

## Objective

Give the agent a first-class todo surface usable anywhere in
chat, and migrate the ceremony todo/priority tools to thin
wrappers so there's a single mutation path.

## Acceptance Criteria

- [ ] New tools registered in `arawn-engine::tools`:
      `todo_create`, `todo_list`, `todo_done`, `todo_undo`,
      `todo_patch`, `todo_archive`, `todo_search`.
- [ ] `todo_create` accepts `{body, workstream?, kind?, due_at?,
      rationale?}`. `kind` defaults to `"user"`; workstream
      defaults to NULL.
- [ ] Existing `daily_add_todo`, `weekly_confirm_priority`,
      `weekly_add_priority` rewritten as thin wrappers calling
      `TodoService` with the appropriate `kind` + ceremony
      `attrs`. Tool surface (names, schemas, return shapes)
      unchanged from the agent's perspective.
- [ ] `ToolCategory` keyword gates updated so `todo`/`reminder`
      surface the new tools in conversational contexts.
- [ ] Per-tool unit tests; agent-loop integration test where the
      agent says "remind me to X" → `todo_create` runs →
      subsequent `todo_list` returns it.

### Risk Considerations

- Tool-surface stability: the agent's prompts reference current
  ceremony tool names. Wrappers must preserve names + arg shapes
  exactly to avoid prompt-engineering regressions in retro /
  daily / weekly flows.

### Dependencies

- Blocked by [[ARAWN-T-0310]] (RPC + events live).
- Blocks [[ARAWN-T-0314]] (TUI consumes these tools),
  [[ARAWN-T-0316]] (UAT uses `todo_done`).

## Status Updates

*To be added during implementation*
