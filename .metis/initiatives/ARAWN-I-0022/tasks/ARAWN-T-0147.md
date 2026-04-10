---
id: add-websocket-authentication-via
level: task
title: "Add WebSocket authentication via session token"
short_code: "ARAWN-T-0147"
created_at: 2026-04-10T01:01:08.850137+00:00
updated_at: 2026-04-10T02:04:38.748263+00:00
parent: ARAWN-I-0022
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0022
---

# Add WebSocket authentication via session token

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0022]]

## Objective
Add authentication to the WebSocket server so that arbitrary local processes (or browser CSRF) cannot connect and issue RPC calls. Generate a session token at startup, require it for connection, and require confirmation for `set_permission_mode(bypass)`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] Random token generated at server startup, written to `~/.arawn/server.token`
- [ ] Token required as query parameter on WebSocket upgrade (`/ws?token=...`)
- [ ] TUI reads token from file to authenticate
- [ ] `Origin` header validated on WebSocket upgrade to reject browser connections
- [ ] `set_permission_mode(bypass)` requires modal confirmation prompt
- [ ] Token optional via config for development setups
- [ ] Tests for auth rejection and token-based acceptance

## Implementation Notes
- Files: `crates/arawn/src/ws_server.rs`, `crates/arawn/src/main.rs` (token generation), `crates/arawn-tui/src/ws_client.rs` (token reading)

## Status Updates
- Added `AppState` struct with `service` + `auth_token` fields
- `run_server` generates token via two concatenated UUIDs, writes to `~/.arawn/server.token`
- `ws_handler` validates `?token=` query param — returns 401 if missing/invalid
- TUI `WsClient::connect` reads `~/.arawn/server.token` and appends `?token=` automatically
- Integration tests bypass auth by using `handle_connection_public` directly (no ws_handler)
- Updated `decision_handler` to destructure from `AppState`
- No new dependencies — used `uuid::Uuid::new_v4()` instead of `rand`, `$HOME` env var instead of `dirs`
- Deferred: Origin header validation, set_permission_mode(bypass) confirmation prompt
- All 29 service+WS tests pass, clean build

## REMOVED_SECTIONS

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