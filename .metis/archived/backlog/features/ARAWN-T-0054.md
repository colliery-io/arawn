---
id: plan-mode-tools-enterplanmode
level: task
title: "Plan mode tools (EnterPlanMode/ExitPlanMode) — read-only planning with approval"
short_code: "ARAWN-T-0054"
created_at: 2026-04-03T01:01:35.838387+00:00
updated_at: 2026-04-04T18:11:27.194573+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Plan mode — observation-only planning with approval

## Objective

Add a plan mode that restricts the agent to **side-effect-free tools** while it researches and designs an approach. Arawn is not just a coding tool — plan mode should work for any planning context: code refactors, project strategy, research, communication plans, infrastructure changes, etc. The agent enters plan mode (via tool call or `/plan` slash command), observes the world, writes a plan to disk, and presents it for user approval before taking any actions.

### Type: Feature | Priority: P2

- **User Value**: Prevents the agent from acting on the wrong approach. User reviews and can edit the plan before any actions are taken. Plans persist across sessions.
- **Effort Estimate**: L

## Key Insight: Observation vs Action, Not Read vs Write

Claude Code's plan mode is file-centric: "don't edit files until I approve." But arawn operates across domains — files, MCP services, APIs, shell commands, external systems. The right abstraction is **observation vs action**:

- **Observation** (side-effect-free): Reading files, searching code, querying databases, listing Jira tickets, checking git status, searching the web, thinking
- **Action** (has side effects): Writing files, running mutations, creating tickets, sending messages, posting to APIs, executing shell commands that modify state

The boundary isn't the tool — it's the operation. `Bash` running `git log` is observation; `Bash` running `git push` is action. An MCP tool that searches Confluence is observation; one that creates a page is action.

## Existing Infrastructure in Arawn

We already have the foundation:

### Tool trait (`arawn-engine/src/tool.rs:45`)
```rust
trait Tool {
    fn is_read_only(&self) -> bool { false }  // already exists
}
```
Currently used for **concurrent execution** — read-only tools run in parallel. This is the right hook but needs reframing.

### Permission system (`arawn-engine/src/permissions/checker.rs`)
```rust
enum ToolCategory { ReadOnly, FileWrite, Shell, Other }
enum PermissionMode { Default, AcceptEdits, BypassPermissions }
```
`ToolCategory::ReadOnly` already classifies: Read, Glob, Grep, Think, Sleep, Task tools. Permission modes use this for fallback decisions. **Plan mode is a natural addition to `PermissionMode`.**

### MCP adapter (`arawn-mcp/src/adapter.rs:70`)
All MCP tools hardcoded to `is_read_only() = false`. This needs to change — MCP spec includes `readOnlyHint` in tool annotations that we should respect.

## Reference: Claude Code Implementation

Claude Code's plan mode has three enforcement layers:

1. **Tool declaration**: `isReadOnly()` on each tool
2. **Permission check**: `mode === 'plan'` blocks non-read-only tools
3. **Filesystem gate**: Write operations check mode, plan file writes explicitly allowed

Key features: plan files at `~/.claude/plans/{slug}.md`, permission stashing (saves pre-plan mode + strips dangerous auto-mode rules), rich approval flow (approve, edit inline, reject with feedback, clear context).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `PermissionMode::Plan` added — only side-effect-free tools allowed
- [ ] `is_read_only()` on Tool trait reused as-is — update doc comment to clarify it means "side-effect-free" (no rename needed)
- [ ] `ToolCategory::ReadOnly` reused as-is — update doc comment to clarify broader meaning
- [ ] MCP tools pass through `readOnlyHint` from tool annotations to `is_read_only()` — defaults to `false` if absent
- [ ] `EnterPlanMode` tool switches engine to plan mode
- [ ] All tools with side effects blocked with clear error in plan mode
- [ ] Exception: writing to the plan file itself is allowed
- [ ] Plan file written to `~/.arawn/plans/{slug}.md`
- [ ] `ExitPlanMode` presents plan for user approval via TUI modal
- [ ] Approval options: approve (proceed), edit (modify plan text), reject with feedback (agent revises)
- [ ] On approval: restore pre-plan permission mode, proceed with execution
- [ ] Plan content preserved in session and on disk for reference
- [ ] `/plan` slash command (I-0014) as alternative entry point
- [ ] Permission state stashing — save and restore pre-plan mode and stripped rules
- [ ] Agent tool propagates plan mode to subagents

## Architecture

### Permission Mode Extension

```rust
enum PermissionMode {
    Default,           // existing
    AcceptEdits,       // existing
    BypassPermissions, // existing
    Plan,              // NEW — only side-effect-free tools allowed
}
```

Plan mode fallback:
```rust
PermissionMode::Plan => match tool_category(tool_name) {
    ToolCategory::SideEffectFree => PermissionDecision::Allowed,
    _ => PermissionDecision::Denied("Plan mode: observation only"),
}
```

### Engine State

```rust
struct PlanModeState {
    active: bool,
    pre_plan_mode: PermissionMode,         // saved for restoration
    stripped_rules: Vec<PermissionRule>,     // dangerous rules removed during planning
    plan_file: Option<PathBuf>,             // ~/.arawn/plans/{slug}.md
    plan_slug: String,                      // human-friendly, cached per session
}
```

### Tool Classification

| Category | Tools | In Plan Mode |
|----------|-------|-------------|
| Side-effect-free | Read, Glob, Grep, Think, Sleep, AskUser, WebSearch, WebFetch, TaskList, TaskGet | Allowed |
| Side-effect-free MCP | Tools with `readOnlyHint: true` in annotations | Allowed |
| File write | Edit, Write | Denied (except plan file) |
| Shell | Bash | Denied |
| Action MCP | Tools without `readOnlyHint` or with `destructiveHint` | Denied |
| Other | Agent, Skill, etc. | Denied |

### MCP readOnlyHint Integration

MCP tools declare hints via annotations in the spec:
```json
{
  "name": "search_issues",
  "annotations": { "readOnlyHint": true }
}
```

The `McpToolAdapter` should read this and return it from `is_side_effect_free()`:
```rust
impl Tool for McpToolAdapter {
    fn is_side_effect_free(&self) -> bool {
        self.mcp_tool.annotations
            .as_ref()
            .and_then(|a| a.read_only_hint)
            .unwrap_or(false)  // conservative default
    }
}
```

### Enforcement Flow

```
Tool execution request
  │
  ├─ Permission checker sees mode == Plan
  ├─ If tool.is_side_effect_free():
  │   └─ Allowed
  ├─ If tool is writing to plan_file path:
  │   └─ Allowed (explicit exception)
  ├─ If tool is EnterPlanMode/ExitPlanMode:
  │   └─ Allowed (meta-tools)
  └─ Otherwise:
      └─ Denied: "Plan mode active — only observation tools allowed. Use ExitPlanMode to present your plan."
```

### Plan File Management

```
~/.arawn/plans/
├── refactor-auth-middleware.md
├── refactor-auth-middleware-agent-abc123.md
├── quarterly-roadmap-review.md          # not just code!
└── migrate-database-to-aurora.md
```

### TUI Approval Modal

When ExitPlanMode fires, TUI shows a modal with:
- Plan content (scrollable, markdown-rendered)
- Action buttons: [Approve] [Edit] [Reject + Feedback]
- Edit mode: inline text editing of plan content, saves to disk
- On approve: engine exits plan mode, restores pre-plan permissions, continues

### Slash Command Integration (I-0014)

`/plan` → invokes EnterPlanMode
`/plan approve` → approves current plan
`/plan edit` → opens plan file for editing
`/plan reject` → rejects with optional feedback

## Implementation Notes

- **No rename needed**: `is_read_only()` and `ToolCategory::ReadOnly` already carry the right semantics. Just update doc comments to clarify they mean "side-effect-free / observation only."
- **Bash in plan mode**: Denied entirely for MVP. Future: could analyze command prefix (`git log` vs `git push`) but that's complex and error-prone.
- **Agent tool**: Should propagate plan mode to subagents so they can't circumvent restrictions.
- **MCP readOnlyHint**: Just pass through the annotation if present. If MCP servers don't declare it, they get `is_read_only() = false`. Their problem.
- **Plan mode is domain-agnostic**: The plan content, slug, and approval flow should not assume code. A plan could be "Steps to migrate our billing provider" or "Communication plan for the outage."