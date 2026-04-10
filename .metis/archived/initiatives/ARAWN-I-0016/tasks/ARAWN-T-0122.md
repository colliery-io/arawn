---
id: agenttaskexecutor-run-queryengine
level: task
title: "AgentTaskExecutor — run QueryEngine for decision tasks within workflow pipelines"
short_code: "ARAWN-T-0122"
created_at: 2026-04-07T21:38:39.807643+00:00
updated_at: 2026-04-09T13:03:00.606257+00:00
parent: ARAWN-I-0016
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0016
---

# AgentTaskExecutor — run QueryEngine for decision tasks within workflow pipelines

## Objective

Implement `AgentTaskExecutor` — a custom cloacina `TaskExecutorTrait` that intercepts decision tasks (those with `arawn_decision` in their cloacina Context) and runs the full arawn QueryEngine loop with workstream context. This bridges cloacina's DAG execution with arawn's agentic reasoning.

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

## Acceptance Criteria

- [ ] `AgentTaskExecutor` struct in `crates/arawn-workflow/src/agent_executor.rs` implementing cloacina's `TaskExecutorTrait`
- [ ] Detects decision tasks by checking for `arawn_decision` key in cloacina `Context<Value>`
- [ ] Creates a fresh arawn session in the specified workstream for each decision execution
- [ ] Injects upstream pipeline data as system prompt preamble, `arawn_decision.prompt` as user message
- [ ] Runs full `QueryEngine::run()` loop with tools, compaction, and workstream-scoped context
- [ ] Writes engine's `final_text` back to cloacina context as the task result
- [ ] Non-decision tasks pass through to cloacina's default `ThreadTaskExecutor`
- [ ] Registered with `DefaultDispatcher` via routing rules
- [ ] Integration test: workflow with data task → decision task → verify decision task runs QueryEngine and result is in context

### Key files
- `crates/arawn-workflow/src/agent_executor.rs` — new file
- Depends on: `arawn-engine` (QueryEngine), `arawn-storage` (Store, sessions), `arawn-llm` (LlmClient)

### Context bridge
- **Into arawn**: `arawn_decision.prompt` → user message, `arawn_decision.workstream` → workstream lookup, upstream context keys → system prompt section
- **Out of arawn**: `engine.run()` returns `final_text` → written to cloacina context as `arawn_decision_result`

### Dependencies
- T-0120 (runner embedded)
- T-0121 (authoring scaffold, for testing with real packages)

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