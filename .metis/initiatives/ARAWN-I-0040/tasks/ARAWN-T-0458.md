---
id: reproduce-and-fix-tui-hang-bug
level: task
title: "Reproduce and fix TUI hang bug using headless test infrastructure"
short_code: "ARAWN-T-0458"
created_at: 2026-03-26T15:26:18.887738+00:00
updated_at: 2026-03-26T16:41:24.919113+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Reproduce and fix TUI hang bug using headless test infrastructure

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0040]]

## Objective

Using the headless test infrastructure from T-0453/T-0454, reproduce the exact bug where the real TUI hangs after sending a chat message. The WS integration tests (ws_integration.rs) proved the backend works — the bug is in the render/event loop or connection timing. Write a test that fails, fix the bug, verify the test passes.

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

## Acceptance Criteria **[REQUIRED]**

- [ ] Reproduction test written: starts noauth server, creates App, sends message via run_headless(), asserts response appears — this test should FAIL before the fix
- [ ] Root cause identified and documented in status updates
- [ ] Fix implemented
- [ ] Reproduction test passes after fix
- [ ] Real TUI works: start server, launch `arawn tui`, type message, response appears
- [ ] Regression test stays in the test suite permanently
- [ ] **GATE: `cargo test --workspace` passes with zero failures before marking complete**

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

### 2026-03-26

**Cannot reproduce in headless tests.** Both scratch and named workstream flows pass — messages send, responses arrive, render correctly. The bug is specific to the real terminal event loop + crossterm interaction.

Tests written:
- `test_reproduce_hang_with_workstream` — creates workstream, switches to it, sends message → PASSES
- `test_chat_in_scratch_workstream` — default scratch context → PASSES

Root cause candidates still open:
1. Groq API latency causing timeout in real usage
2. macOS-specific crossterm event stream behavior
3. Terminal emulator interaction (WezTerm-specific?)
4. The `process_tick()` refactor may have fixed a subtle ordering issue as a side effect

**Next step: user needs to rebuild and test the real TUI to see if the refactoring fixed it.**