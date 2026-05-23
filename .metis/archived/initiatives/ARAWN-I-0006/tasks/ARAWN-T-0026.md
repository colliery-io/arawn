to ---
id: localservice-arawnservice-impl
level: task
title: "LocalService — ArawnService impl with engine + store + streaming bridge"
short_code: "ARAWN-T-0026"
created_at: 2026-04-01T10:39:18.954192+00:00
updated_at: 2026-04-01T10:39:18.954192+00:00
parent: ARAWN-I-0006
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0006
---

# LocalService — ArawnService impl with engine + store + streaming bridge

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0006]]

## Objective

Implement `LocalService` that wraps the engine, store, tool registry, and compactor behind the `ArawnService` trait. The streaming bridge converts engine session mutations into `EngineEvent` streams via a tokio channel.

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

- [ ] `LocalService` struct: owns `Store`, `Arc<dyn LlmClient>`, `Arc<ToolRegistry>`, `ModelLimits`, system prompt
- [ ] `LocalService::new(store, llm, registry, config)` constructor
- [ ] Implements all `ArawnService` methods
- [ ] CRUD methods delegate to Store (list/create workstreams, sessions)
- [ ] `send_message`: spawns background task, runs QueryEngine::run, emits EngineEvent stream via mpsc channel
- [ ] Streaming bridge: intercepts session message appends and maps to EngineEvent variants (text → StreamingText, tool_use → ToolCallStart, tool_result → ToolCallResult, final → Complete)
- [ ] Messages persisted to JSONL during the engine run (same as current binary behavior)
- [ ] `cancel`: sets a cancellation flag checked by the engine task
- [ ] Refactor main.rs one-shot CLI to use LocalService instead of direct engine wiring
- [ ] All existing tests still pass (backward compat)
- [ ] Unit test: LocalService CRUD methods work with in-memory store
- [ ] Unit test: send_message returns stream that yields events

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

## Implementation Notes

- `local_service.rs` in `crates/arawn/src/` (binary crate, since it wires all internal crates)
- The streaming bridge is the hardest part. The engine currently mutates `Session` directly. We need to wrap session message appends so each one also emits an `EngineEvent` to the channel. Options: (a) callback on Session, (b) wrapper struct that intercepts add_message, (c) diff the session before/after each engine turn. Option (b) is cleanest.
- `cancel` needs a shared `AtomicBool` or `CancellationToken` passed into the engine task. The engine checks it each loop iteration.
- Depends on: T-0025 (ArawnService trait)

## Status Updates
*To be added during implementation*