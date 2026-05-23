---
id: memory-slash-commands-remember
level: task
title: "Memory slash commands — /remember, /memory, /forget in TUI command system"
short_code: "ARAWN-T-0101"
created_at: 2026-04-05T14:41:56.572665+00:00
updated_at: 2026-04-05T16:08:05.468960+00:00
parent: ARAWN-I-0015
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0015
---

# Memory slash commands — /remember, /memory, /forget in TUI command system

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0015]]

## Objective **[REQUIRED]**

Add memory-related slash commands to the TUI command system (I-0014). `/remember` for explicit fact storage, `/memory` for KB overview, `/forget` for deletion. These give the user direct control over what the agent remembers.

### Type: Feature | Priority: P2 | Effort: S

## Depends On: T-0097 (memory_store), T-0098 (memory_search), I-0014 (slash command system — already done)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `/remember <text>` — parses text, stores as entity via `memory_store` with confidence Stated (1.0). Infers entity_type from content (default: Fact). Shows confirmation.
- [ ] `/memory` — queries both KB tiers, shows summary as markdown table: entity counts by type, total entities, most recent entities. Displayed via existing inventory modal/system message pattern.
- [ ] `/forget <id or title>` — searches KB for matching entity, marks as superseded or deletes. Requires confirmation before deletion.
- [ ] Commands registered in TUI `CommandRegistry` with autocomplete support
- [ ] Server-side WS methods: `remember_fact`, `memory_summary`, `forget_entity` routed to `MemoryManager`
- [ ] Unit tests: command parsing, WS round-trip for each command

## Implementation Notes

- Follows same pattern as existing `/tools`, `/skills` inventory commands
- `/remember` routes through the server to `memory_store` tool logic (search-before-create)
- `/memory` is a read-only query — count entities by type in both stores, format as table
- `/forget` needs fuzzy matching — search by ID prefix or title substring, present candidates if ambiguous
- `/remember` should be smart about entity_type inference: "I prefer..." → Preference, "The convention is..." → Convention, "We decided..." → Decision, otherwise Fact

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