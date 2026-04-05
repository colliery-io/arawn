---
id: git-worktree-isolation-for-agents
level: task
title: "Git worktree isolation for agents (EnterWorktree/ExitWorktree)"
short_code: "ARAWN-T-0053"
created_at: 2026-04-03T01:01:34.469600+00:00
updated_at: 2026-04-03T01:01:34.469600+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Git worktree isolation for agents (EnterWorktree/ExitWorktree)

## Objective

Add `isolation: "worktree"` parameter to the AgentTool that creates a temporary git worktree so the sub-agent works on an isolated copy of the repo. Changes don't affect the parent's working directory. Worktree is auto-cleaned if no changes; preserved with branch name if changes were made.

### Type: Feature | Priority: P2

- **User Value**: Safe parallel agent work without file conflicts. Essential for large refactors.
- **Effort Estimate**: M

## Acceptance Criteria

## Acceptance Criteria

- [ ] `isolation: "worktree"` param on AgentTool creates git worktree before running
- [ ] Sub-agent's working_dir points to worktree
- [ ] Auto-cleanup if agent made no changes; preserved with branch name if changes exist
- [ ] Parent working directory unaffected
- [ ] Error if not in a git repo

## Implementation Notes

- `git worktree add .arawn/worktrees/<agent-id> -b worktree-<agent-id>`
- Override `ToolContext.working_dir` to worktree path
- On completion: `git diff --quiet` to check for changes
- Reference: Claude Code's `createAgentWorktree`/`removeAgentWorktree`
- Depends on: AgentTool (done)