---
id: system-prompt-builder-dynamic
level: task
title: "System prompt builder — dynamic context injection (cwd, git info, env, workstream)"
short_code: "ARAWN-T-0041"
created_at: 2026-04-01T11:02:05.713942+00:00
updated_at: 2026-04-02T12:35:21.919543+00:00
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

# System prompt builder — dynamic context injection (cwd, git info, env, workstream)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

Replace the static system prompt string with a dynamic builder that assembles context from multiple sources. Currently main.rs hardcodes "You are Arawn... working directory is {cwd}" and that's it. Claude Code constructs a layered system prompt including: persona, available tools summary, user context (cwd, OS, shell), git status, session metadata, project files, and memory. A richer system prompt produces dramatically better tool selection and behavior.

### Priority
- P2 — the system prompt is the single highest-leverage improvement for response quality

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `SystemPromptBuilder` struct with builder pattern: `.persona(...)`, `.working_dir(...)`, `.workstream(...)`, `.git_info(...)`, `.context_files(...)`, `.memories(...)`, `.build() → String`
- [ ] **Persona section**: "You are Arawn, a personal agentic assistant..." (base identity, tone, capabilities)
- [ ] **Environment section**: OS, shell, working directory, date/time
- [ ] **Workstream section**: current workstream name, root directory, description if available
- [ ] **Git section**: current branch, dirty status, recent commits (if in a git repo — run `git status` + `git log --oneline -5` at session start)
- [ ] **Tools section**: brief summary of available tools and when to use each (supplements tool descriptions)
- [ ] **Context files section**: injected `.arawn.md` content (depends on T-0037, can be empty initially)
- [ ] **Memories section**: injected relevant memories (depends on T-0038, can be empty initially)
- [ ] Builder produces a single string with clear section headers
- [ ] Token budget: builder tracks estimated tokens and truncates lower-priority sections if approaching a budget (e.g., 4k tokens for system prompt)
- [ ] Engine uses `SystemPromptBuilder` instead of the hardcoded string in `QueryEngineConfig`
- [ ] Test: builder produces expected sections
- [ ] Test: missing optional sections (no git, no context file) handled gracefully
- [ ] Test: token budget truncation works

## Implementation Notes

- `system_prompt.rs` in `crates/arawn-engine/src/`
- Git info: run `git rev-parse --is-inside-work-tree`, `git branch --show-current`, `git status --short` at session start via `tokio::process::Command`. Cache the result — don't re-run every turn.
- The builder is called once at session start, not per-turn. If the workstream changes (promotion), rebuild.
- Sections ordered by priority: persona (always), environment (always), workstream (always), git (if available), tools (always), context files (if available), memories (if available). Truncation removes from the bottom.
- Reference: Claude Code's context building in `src/context/` directory
- Depends on: nothing for base implementation. T-0037 and T-0038 add optional sections later.

## Test Scenarios

### Assembly
- [ ] TC-01: Default assembly — no overrides, all 7 static + dynamic sections present in correct order
- [ ] TC-02: Section headers — each section has a clear `# Section Name` header in output
- [ ] TC-03: Empty optional sections — memories empty, plugin prompts empty, no session context → no empty headers in output

### Override System
- [ ] TC-04: Single section override — place identity.md in prompts dir, verify it replaces compiled default
- [ ] TC-05: Partial overrides — override identity.md only, other 6 sections use compiled defaults
- [ ] TC-06: Missing override dir — no ~/.arawn/prompts/ exists, falls back to all defaults gracefully
- [ ] TC-07: Empty override file — user places empty identity.md, section content should be empty (not default)

### Token Budget
- [ ] TC-08: Under budget — normal build stays under 6k tokens, all sections included
- [ ] TC-09: Over budget — build with tiny budget (500 tokens), verify low-priority sections dropped
- [ ] TC-10: Priority ordering — identity (P0) survives budget cuts, plugin prompts (P9) dropped first
- [ ] TC-11: Truncation not corruption — dropped sections are cleanly removed, no partial section content

### Context Files
- [ ] TC-12: .arawn.md present — content injected into context files section
- [ ] TC-13: .arawn.md missing — section omitted cleanly
- [ ] TC-14: Large .arawn.md — truncated with 70% head + 20% tail, truncation marker present

### Dynamic Content
- [ ] TC-15: Tools section reflects registry — pass different tool lists, verify using_tools section changes
- [ ] TC-16: Per-turn freshness — build twice with different tool names, outputs differ
- [ ] TC-17: Environment section — contains OS, shell, cwd, date, model name
- [ ] TC-18: Workstream section — contains workstream name and root dir

### Snapshot
- [ ] TC-19: Full build snapshot — typical state with all sections, insta snapshot for regression

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates
- **2026-04-02**: Implementation complete. SystemPromptBuilder with 7 static sections (compiled defaults, overridable via ~/.arawn/prompts/{name}.md) + 6 dynamic sections (environment, workstream, tools, context_files, memories, plugin_prompts). Token budget enforcement (6k default, priority-based section dropping). Context file loading with 70/20 head-tail truncation. PromptContext on QueryEngineConfig for per-turn prompt building. DataLayout adds prompts/ dir. Binary wired for both serve and CLI modes. All 19 test scenarios (TC-01 through TC-19) implemented + 1 insta snapshot. 255 total tests, clippy clean.