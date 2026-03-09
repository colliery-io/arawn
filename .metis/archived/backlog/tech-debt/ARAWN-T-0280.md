---
id: add-arawn-client-api-endpoint
level: task
title: "Add arawn-client API endpoint tests using wiremock"
short_code: "ARAWN-T-0280"
created_at: 2026-03-08T03:17:25.010273+00:00
updated_at: 2026-03-08T15:24:35.060898+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Add arawn-client API endpoint tests using wiremock

## Objective

`arawn-client` has **4 tests for 30+ public API methods**. It already has `wiremock 0.6` as a dev-dependency but doesn't use it. Write comprehensive tests for every API module using wiremock to mock HTTP responses.

### Priority
- [x] P1 - High (client is used by CLI and TUI — bugs here surface as user-facing issues)
- **Size**: L

### Current Problems
- API methods completely untested — request serialization, response parsing, error handling all unverified
- `wiremock` dev-dependency available but unused
- SSE streaming (`ChatApi::stream()`) untested
- Auth header injection untested
- Error responses (4xx, 5xx) untested

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Every API module has a test file in `tests/` or inline `#[cfg(test)]`
- [ ] Each API method tested for: happy path, error response (4xx), server error (5xx)
- [ ] Request body/query params verified via wiremock matchers
- [ ] Auth header (`Bearer <token>`) verified on every request
- [ ] SSE streaming tested with multi-chunk mock responses
- [ ] Error type mapping verified (HTTP status → client error variant)
- [ ] At least 60 new test functions total

## Implementation Notes

### Modules to test (by file, with method count)

| File | Methods | Tests Needed |
|------|---------|-------------|
| `api/agents.rs` | 3 | list, get, main + errors |
| `api/chat.rs` | 4 | send, message, stream, stream_message + errors |
| `api/sessions.rs` | 5 | list, get, create, update, delete + errors |
| `api/workstreams.rs` | 4 | list, get, create, get_messages + errors |
| `api/notes.rs` | 6 | list, get, create, update, delete, search + errors |
| `api/memory.rs` | 3 | search, store, delete + errors |
| `api/tasks.rs` | 5 | list, get, create, update, delete + errors |
| `api/mcp.rs` | 4 | list_servers, add_server, remove_server, list_tools + errors |
| `api/health.rs` | 2 | check, is_healthy + errors |
| `api/config.rs` | 1 | get + errors |
| `types.rs` | — | Request/response serde roundtrip tests |

### Test pattern

```rust
#[tokio::test]
async fn test_sessions_list() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/sessions"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"id": "sess-1", "workstream_id": "ws-1", ...}
        ])))
        .mount(&mock_server)
        .await;

    let client = ArawnClient::new(&mock_server.uri(), Some("test-token"));
    let sessions = client.sessions().list().await.unwrap();
    assert_eq!(sessions.len(), 1);
}
```

### Dependencies
- None (wiremock already in Cargo.toml)

## Status Updates

### Completed
- Created 4 test files with 92 new wiremock tests (96 total including 4 existing):
  - `tests/api_health_config_agents.rs` — 14 tests (health, config, agents)
  - `tests/api_sessions.rs` — 18 tests (list, get, create, update, delete, messages, auth)
  - `tests/api_workstreams_chat.rs` — 24 tests (workstreams CRUD, messages, sessions, promote; chat send, stream SSE)
  - `tests/api_notes_memory_tasks_mcp.rs` — 36 tests (notes CRUD, memory search/store, tasks list/cancel, MCP servers/tools/connect)
- Every API module covered: health, config, agents, sessions, chat, workstreams, notes, memory, tasks, mcp
- Each method tested for: happy path, error responses (404, 500, 401 where applicable)
- Auth header verified via wiremock matchers
- SSE streaming tested with multi-event mock
- Request body/query params verified via wiremock matchers
- All 96 tests pass