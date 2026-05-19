---
id: decide-skill-tool-semantics
level: task
title: "Decide skill tool semantics — implement sub-conversation or drop unused frontmatter fields"
short_code: "ARAWN-T-0351"
created_at: 2026-05-19T12:07:49.161718+00:00
updated_at: 2026-05-19T12:07:49.161718+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Decide skill tool semantics — implement sub-conversation or drop unused frontmatter fields

## Objective

Decide what the `skill` tool actually does and align code + docs. Today the tool reads in a skill's `allowed-tools` and `model` frontmatter fields but **doesn't enforce them** — it just returns the rendered prompt body as the tool result and the parent agent continues with its own permission / LLM settings.

## Impact

- **Severity:** P2 — feature drift. Frontmatter fields look enforceable but aren't. Users authoring a skill with `allowed-tools: ["file_read"]` think they're scoping the skill's execution, but the parent agent has its full toolset available.
- Either implement the enforcement (the documented semantics) or drop the fields (current code reality).

## Proposed options

### Option A — implement sub-conversation semantics

The `skill` tool spawns a focused agent loop (similar to the `agent` tool's `subagent_type` path) with the skill's `allowed-tools` and `model` enforced. Returns the sub-agent's final response. Cost: real implementation work; needs care around context budget (sub-conversations can balloon).

### Option B — drop the unused fields

Remove `allowed-tools`, `model`, `argument-hint` from `SkillDefinition` (or keep `argument-hint` since it's user-visible in autocomplete). Document `skill` as a pure prompt-injector and refer users to the `agent` tool (with `subagent_type`) when isolation is needed.

Recommend **Option A** for the long term — skills are most useful when scoped. Option B is the cheap path if A is too big right now.

## Implementation notes

- `crates/arawn-engine/src/tools/skill.rs:54-95` — `SkillTool::execute` just renders + returns the body.
- `crates/arawn-engine/src/skills/definition.rs` — `SkillDefinition` struct fields.
- Option A could reuse the `AgentTool` machinery — a skill is essentially an agent definition with a fixed system prompt.
- Whichever option ships, update `docs/src/reference/skills.md` to match.

## Acceptance criteria

- [ ] Decision recorded (A or B).
- [ ] Code + docs align — either the tool enforces frontmatter, or the frontmatter fields are removed/clarified.
- [ ] Built-in skills (`workflows.md`, `workstream-create.md`) verified to behave the same after the change.

Also tied to the `skill` tool's input field naming — code uses `skill` and `args`; docs were previously wrong (said `name` / `arguments`). The triple-check (commit 7037763) corrected the docs; this task tracks the larger semantics decision.

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

## Acceptance Criteria **[REQUIRED]**

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

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

## Status Updates **[REQUIRED]**

*To be added during implementation*