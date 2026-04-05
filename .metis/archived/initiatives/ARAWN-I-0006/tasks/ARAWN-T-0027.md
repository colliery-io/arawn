---
id: websocket-server-axum-json
level: task
title: "WebSocket server — axum, JSON protocol, arawn serve command"
short_code: "ARAWN-T-0027"
created_at: 2026-04-01T10:39:20.033489+00:00
updated_at: 2026-04-01T11:37:57.235153+00:00
parent: ARAWN-I-0006
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0006
---

# WebSocket server — axum, JSON protocol, arawn serve command

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0006]]

## Objective

Build a WebSocket server using axum that exposes the `ArawnService` over JSON-RPC-style messages. Add `arawn serve` command to the binary that starts the daemon on localhost.

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

- [ ] `ws_server.rs` in binary crate with axum WebSocket handler
- [ ] Single endpoint: `ws://localhost:3100/ws`
- [ ] JSON protocol: request `{"id": N, "method": "...", "params": {...}}`, response `{"id": N, "result": ...}`
- [ ] Streaming events: `{"event": "streaming_text", "data": {"text": "..."}}`
- [ ] Supported methods: `list_workstreams`, `create_workstream`, `list_sessions`, `create_session`, `load_session`, `send_message`, `cancel`
- [ ] `arawn serve` command starts the server, logs to stderr, runs until Ctrl-C
- [ ] `arawn serve --port 3200` configurable port (default 3100)
- [ ] Server holds `Arc<LocalService>` shared across connections
- [ ] Multiple concurrent WebSocket connections work
- [ ] One-shot CLI (`arawn "prompt"`) still works unchanged
- [ ] Manual test: connect with `websocat` or similar, send list_workstreams, get response

## Implementation Notes

- `ws_server.rs` in `crates/arawn/src/`
- Dependencies: `axum` with `ws` feature, `tower`, `tokio`
- Message dispatch: parse incoming JSON, match `method` field, call LocalService, serialize response
- For `send_message`: spawn task that polls the EngineEvent stream and sends each event as a WS frame
- Error responses: `{"id": N, "error": {"code": "...", "message": "..."}}`
- Binary arg parsing: detect `serve` as first arg before the existing prompt logic
- Depends on: T-0026 (LocalService)

## Status Updates
- **2026-04-01**: Complete. ws_server.rs with axum 0.8 WebSocket handler. JSON-RPC protocol: request {id, method, params} → response {id, result/error}. All 7 ArawnService methods dispatched. send_message streams EngineEvents as individual WS frames. Error handling for parse errors, unknown methods, missing params. `arawn serve --port N` wired into main.rs (default 3100). Server verified: starts, listens, logs correctly. One-shot CLI unchanged. 161 tests, clippy clean.