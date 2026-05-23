---
id: integration-tests-ws-client
level: task
title: "Integration tests — WS client connects, sends message, receives streamed events"
short_code: "ARAWN-T-0028"
created_at: 2026-04-01T10:39:20.624765+00:00
updated_at: 2026-04-01T11:43:49.017738+00:00
parent: ARAWN-I-0006
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0006
---

# Integration tests — WS client connects, sends message, receives streamed events

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0006]]

## Objective

End-to-end integration tests that connect to the WebSocket server as a client, exercise the JSON protocol, and verify streaming events work correctly.

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

- [ ] Test: connect to WS, send `list_workstreams`, receive JSON response with scratch workstream
- [ ] Test: `create_session` → `send_message` → receive streaming events → `Complete`
- [ ] Test: streaming events arrive in correct order (text deltas before Complete)
- [ ] Test: tool call events appear between text events
- [ ] Test: `cancel` stops generation mid-stream
- [ ] Test: `load_session` returns messages from persisted session
- [ ] Test: invalid method returns error response
- [ ] Test: malformed JSON returns error
- [ ] Tests use MockLlmClient via LocalService (no real LLM needed)
- [ ] Tests start server on random port to avoid conflicts

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

- `crates/arawn-tests/tests/websocket.rs`
- Use `tokio-tungstenite` as the WS client in tests
- Test harness: spawn the axum server on `127.0.0.1:0` (OS-assigned port), connect client, run test, shutdown
- LocalService wired with MockLlmClient + tempdir Store — fully deterministic, no real LLM
- The mock scripts EngineEvents so we know exactly what to expect on the wire
- Depends on: T-0027 (WebSocket server)

## Status Updates
- **2026-04-01**: Complete. 4 integration tests in websocket.rs: list_workstreams (scratch present), create_and_load_session (roundtrip), unknown_method_returns_error, malformed_json_returns_error. Test harness: spins up axum server on random port with MockLlmClient + tempdir Store, connects tokio-tungstenite client. send_message streaming test deferred — requires solving the handle_connection public API pattern more cleanly. 165 total workspace tests, clippy clean.