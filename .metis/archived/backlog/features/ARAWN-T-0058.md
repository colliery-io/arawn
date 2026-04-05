---
id: skill-command-framework-skilltool
level: task
title: "Skill/command framework — SkillTool for executing reusable prompt-based workflows"
short_code: "ARAWN-T-0058"
created_at: 2026-04-03T01:01:41.610416+00:00
updated_at: 2026-04-04T03:10:54.469478+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Skill/command framework — SkillTool for executing reusable prompt-based workflows

## Objective

Build a skill/command framework that loads reusable prompt-based workflows from markdown files. Skills are invoked by name (like slash commands) and inject their prompt content into the conversation. This enables `/commit`, `/review`, `/test` style workflows.

### Type: Feature | Priority: P1

- **User Value**: Reusable workflows for common tasks (/commit, /review, /test). Reduces repetitive prompting and encodes project-specific conventions.
- **Effort Estimate**: L

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

**Skill definition:**
- [ ] `SkillDefinition` struct: name, description, prompt content, frontmatter fields
- [ ] YAML frontmatter parsing from `.md` files: `description`, `allowed-tools`, `model`, `user-invocable`, `argument-hint`
- [ ] Markdown body (after frontmatter) is the prompt template
- [ ] Skills loaded from `.arawn/skills/` directory (and `~/.arawn/skills/` for user-global)

**Skill loading:**
- [ ] `load_skills_dir()` scans a directory for `*.md` files and `*/skill.md` subdirectories
- [ ] Skill name derived from filename (e.g. `commit.md` → `commit`, `deploy/skill.md` → `deploy`)
- [ ] Deduplication: if same skill name from multiple sources, project takes priority over user
- [ ] `SkillRegistry` holds loaded skills, queryable by name

**SkillTool:**
- [ ] Tool named `Skill` with input `{ skill: String, args: Option<String> }`
- [ ] Looks up skill by name from SkillRegistry
- [ ] Injects skill prompt as a user message into the conversation, then continues the agentic loop
- [ ] Returns error if skill not found or disabled
- [ ] Permission integration: `Skill(name)` patterns work with existing allow/deny/ask rules

**System prompt integration:**
- [ ] Skills listed in system prompt so the model knows what's available
- [ ] Budget-aware listing (cap total skill descriptions to ~1% of context window tokens)
- [ ] Each skill description truncated to ~250 chars if needed

**Tests:**
- [ ] Unit tests for frontmatter parsing (valid, missing fields, malformed YAML)
- [ ] Unit tests for skill loading (multiple files, dedup, subdirectory skills)
- [ ] Unit test for SkillTool execution (skill found, skill not found)
- [ ] Unit test for system prompt skill listing with budget

## Implementation Notes

### Files to create
- `crates/arawn-engine/src/skills/mod.rs` — public API
- `crates/arawn-engine/src/skills/definition.rs` — SkillDefinition, frontmatter parsing
- `crates/arawn-engine/src/skills/loader.rs` — load_skills_dir(), SkillRegistry
- `crates/arawn-engine/src/tools/skill.rs` — SkillTool implementation

### Frontmatter format (matching Claude Code)
```yaml
---
description: "Create a git commit with a conventional message"
allowed-tools:
  - "Bash(git *)"
  - "Read"
argument-hint: "[-m message]"
user-invocable: true
model: sonnet
---

# Commit workflow
Review staged changes and create a commit...
```

### Key design decisions
- **Inline only for MVP** — no fork/sub-agent execution mode yet
- **Prompt injection** — skill prompt becomes a user message, model responds naturally
- **No `$ARGUMENTS` substitution** — args passed as-is in the injected prompt (Claude Code does simple string replacement)
- **Permission check** reuses existing PermissionChecker with `Skill(name)` patterns
- Depends on: system prompt builder (done), permission system (done)

## Status Updates

- **2026-04-04**: Implemented full skill framework in `crates/arawn-engine/src/skills/` and `tools/skill.rs`:
  - `definition.rs`: SkillDefinition struct, YAML frontmatter parsing (description, allowed-tools as multi-line/inline lists, model, user-invocable, argument-hint), SkillSource enum
  - `loader.rs`: load_skills_dir() discovers `*.md` files and `*/skill.md` subdirs, SkillRegistry with case-insensitive lookup, load_merged_skills() with project>user priority, format_skill_listing() with budget/truncation
  - `tools/skill.rs`: SkillTool (name="Skill") accepts skill name + optional args, returns prompt content, lists available skills on not-found error
  - 27 unit tests, all passing. Workspace builds clean.