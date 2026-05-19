---
id: decide-skill-tool-semantics
level: task
title: "Decide skill tool semantics — implement sub-conversation or drop unused frontmatter fields"
short_code: "ARAWN-T-0351"
created_at: 2026-05-19T12:07:49.161718+00:00
updated_at: 2026-05-19T17:35:29.982845+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Skill tool: render frontmatter into the skill body

## Objective

Make the `skill` tool's `allowed-tools` and `model` frontmatter
match how Anthropic's skill system actually works: render the
metadata into the tool output as advisory guidance the model
self-complies with. Today the fields are parsed but never reach
the model — they look enforceable, but in practice they're dead
state.

Locked design decisions (from the T-0351 discussion, 2026-05-19):

- **Don't build a sub-conversation runtime.** Anthropic's skill
  system uses prompt engineering, not runtime gating. The `agent`
  tool with `subagent_type` already provides real isolation when
  it's needed.
- **Don't drop the fields either.** Skills authored for the
  Claude Code ecosystem use `allowed-tools` and `model`; arawn
  silently dropping them creates a divergence with the upstream
  format.
- **Render the metadata into the skill body.** When the skill
  tool fires, if the definition declares `allowed-tools` or
  `model`, append a short "Skill constraints" block to the
  tool output. The model reads it and self-complies. This is
  the actual contract Anthropic's system offers.

## Surface

The rendered output goes from this (today):

```
[Skill body markdown...]

Arguments: <args>          # (if args were passed)
```

…to this (after T-0351):

```
[Skill body markdown...]

Arguments: <args>          # (if args were passed)

---
Skill constraints (advisory — the agent should self-comply):
- allowed-tools: Bash(git *), Read
- recommended model: claude-haiku-4
```

The constraints block is omitted entirely when both
`allowed_tools` and `model` are `None`.

## Acceptance criteria

- [x] `SkillTool::execute` appends a "Skill constraints" footer
  when the definition declares `allowed_tools` or `model`;
  omitted otherwise.
- [x] One line per field with a stable prefix.
- [x] Unit tests cover all four rendering paths (none / tools
  only / model only / both) plus footer ordering after args.
- [x] `docs/src/reference/skills.md` updated to describe the
  advisory semantics and point at the `agent` tool for real
  isolation.
- [x] No changes to `SkillDefinition` or `parse_skill_markdown`.
- [x] `angreal test unit` green. `angreal check workspace` green.

## Implementation notes

- All changes in `crates/arawn-engine/src/tools/skill.rs`. Body
  rendering switches from a single `if args.is_empty()` branch
  to a small builder that appends args + constraints in order.
- Built-in skills (`workflows`, `workstream-create`, etc.)
  don't currently set `allowed_tools` or `model`, so their
  output is unaffected.

Surfaced during ARAWN-I-0051 doc triple-check; redesigned in
the T-0351 discussion 2026-05-19 once the Anthropic-skill
compatibility framing surfaced.

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

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

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