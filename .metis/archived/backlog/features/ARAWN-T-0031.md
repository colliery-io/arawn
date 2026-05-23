---
id: improve-tool-descriptions-detailed
level: task
title: "Improve tool descriptions — detailed prompts that guide LLM behavior"
short_code: "ARAWN-T-0031"
created_at: 2026-04-01T10:57:17.755557+00:00
updated_at: 2026-04-02T12:39:51.917993+00:00
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

# Improve tool descriptions — detailed prompts that guide LLM behavior

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

Replace our one-liner tool descriptions with detailed, behavior-guiding prompts modeled on Claude Code's approach. Current descriptions are functional but don't steer the LLM — Claude Code's descriptions are mini-manuals that actively direct tool selection, usage patterns, and error avoidance.

### Priority
- P2 — improves quality of tool usage but not blocking

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] ShellTool: working dir persistence, prefer dedicated tools over shell, timeout guidance, multiple commands, avoid sleep loops
- [ ] FileReadTool: absolute vs relative paths, line limits, offset guidance, can't read directories
- [ ] FileWriteTool: creates parent dirs, path traversal note, prefer FileEdit for modifications
- [ ] FileEditTool: exact string match requirement, ambiguity handling, use for modifications not full rewrites
- [ ] GrepTool: regex syntax, glob/type filters, "ALWAYS use Grep, NEVER invoke grep via shell", output modes
- [ ] ThinkTool: when to use (reasoning scratchpad), no side effects
- [ ] All descriptions reference other tools by name (e.g., "use FileRead instead of cat via shell")

## Implementation Notes

- Reference: `claude-code/src/tools/*/prompt.ts` for each tool's description
- Descriptions are returned from `Tool::description()` — just update the string literals
- Trade-off: longer descriptions = more tokens per request. Claude Code sends ~50 lines for BashTool alone. We need to balance guidance with token budget, especially on smaller context models.
- Consider: move descriptions to a `prompt.rs` module per tool (like Claude Code's `prompt.ts`) to separate prompt engineering from tool logic

## Status Updates
*To be added during implementation*