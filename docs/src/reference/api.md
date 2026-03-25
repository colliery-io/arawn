# REST API Reference

HTTP endpoints for interacting with the Arawn server.

## Base URL

```
http://localhost:8080
```

The port and bind address are configurable via `[server]` config or `arawn start --port`.

## Authentication

All `/api/v1/*` endpoints require a bearer token. Health, metrics, OpenAPI, and WebSocket upgrade endpoints do not.

```
Authorization: Bearer <token>
```

Set the token via `ARAWN_API_TOKEN` environment variable or `arawn start --token`.

## Pagination

All list endpoints support pagination via query parameters.

| Parameter | Type | Default | Max | Description |
|-----------|------|---------|-----|-------------|
| `limit` | integer | 50 | 100 | Maximum items to return |
| `offset` | integer | 0 | | Number of items to skip |

## Error Response Format

All errors return a consistent JSON structure.

```json
{
  "error": {
    "code": "invalid_request",
    "message": "Session not found",
    "details": {}
  }
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `invalid_request` | 400 | Malformed request body or parameters |
| `unauthorized` | 401 | Missing or invalid authentication token |
| `not_found` | 404 | Resource does not exist |
| `rate_limited` | 429 | Too many requests from this IP |
| `internal_error` | 500 | Unexpected server error |

## Rate Limiting

Rate limiting is per-IP when enabled (`[server] rate_limiting = true`). Default: 120 requests per minute.

Response headers on 429:

```
HTTP/1.1 429 Too Many Requests
Retry-After: 60
X-RateLimit-Limit: 120
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1705323600
```

---

## Health (No Auth)

### GET /health

Simple health check.

**Response** `200 OK`:

```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

### GET /health/deep

Deep health check with subsystem status. Verifies connectivity to memory store, embedding provider, and other subsystems.

**Response** `200 OK`:

```json
{
  "status": "ok",
  "version": "0.1.0",
  "checks": [
    { "name": "memory_store", "status": "ok" },
    { "name": "embedding", "status": "ok" },
    { "name": "mcp", "status": "warn", "detail": "1 of 3 servers unreachable" }
  ]
}
```

Check statuses: `ok`, `warn`, `error`, `unavailable`.

### GET /metrics

Basic runtime metrics.

**Response** `200 OK`:

```json
{
  "uptime_secs": 3600,
  "active_sessions": 3,
  "memory": {
    "total_memories": 150,
    "total_notes": 12
  }
}
```

---

## OpenAPI Documentation (No Auth)

### GET /api/docs

Interactive Swagger UI for exploring the API.

### GET /api/openapi.json

OpenAPI 3.0 specification in JSON format.

---

## Chat (Auth Required)

### POST /api/v1/chat

Synchronous chat. Sends a message and waits for the complete response.

**Request:**

```json
{
  "message": "Explain ownership in Rust",
  "session_id": "optional-session-id",
  "workstream_id": "optional-workstream-id"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `message` | string | yes | The user message |
| `session_id` | string | no | Existing session to continue |
| `workstream_id` | string | no | Workstream context |

**Response** `200 OK`:

```json
{
  "session_id": "ses_abc123",
  "response": "Ownership in Rust is a set of rules...",
  "tool_calls": [],
  "usage": {
    "input_tokens": 42,
    "output_tokens": 256
  }
}
```

### POST /api/v1/chat/stream

Streaming chat via Server-Sent Events (SSE). Same request body as synchronous chat.

**Response:** `200 OK` with `Content-Type: text/event-stream`.

Event types:

| Event | Data Fields | Description |
|-------|------------|-------------|
| `text` | `{"text": "..."}` | Text chunk from the agent |
| `tool_start` | `{"id": "t1", "name": "shell"}` | Tool execution started |
| `tool_end` | `{"id": "t1", "result": "..."}` | Tool execution completed |
| `done` | `{"usage": {"input_tokens": N, "output_tokens": N}}` | Stream complete |

Example stream:

```
event: text
data: {"text": "Ownership in Rust "}

event: tool_start
data: {"id": "t1", "name": "shell"}

event: tool_end
data: {"id": "t1", "result": "..."}

event: done
data: {"usage": {"input_tokens": 42, "output_tokens": 256}}
```

---

## Sessions (Auth Required)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/sessions` | Create a new session |
| GET | `/api/v1/sessions` | List sessions |
| GET | `/api/v1/sessions/{id}` | Get session details |
| PATCH | `/api/v1/sessions/{id}` | Update session metadata |
| DELETE | `/api/v1/sessions/{id}` | Delete session (triggers background indexing) |
| GET | `/api/v1/sessions/{id}/messages` | Get session conversation history |

### POST /api/v1/sessions

**Request:**

```json
{
  "workstream_id": "optional-workstream-id"
}
```

**Response** `201 Created`:

```json
{
  "id": "ses_abc123",
  "created_at": "2025-01-10T08:00:00Z",
  "message_count": 0
}
```

### GET /api/v1/sessions

**Query parameters:** `limit`, `offset`.

**Response** `200 OK`:

```json
{
  "sessions": [
    {
      "id": "ses_abc123",
      "created_at": "2025-01-10T08:00:00Z",
      "message_count": 5
    }
  ],
  "total": 42
}
```

### GET /api/v1/sessions/{id}

**Response** `200 OK`: Full session detail object.

### PATCH /api/v1/sessions/{id}

**Request:**

```json
{
  "title": "Updated session title"
}
```

**Response** `200 OK`: Updated session object.

### DELETE /api/v1/sessions/{id}

**Response** `204 No Content`. Triggers background memory indexing of the session.

### GET /api/v1/sessions/{id}/messages

**Response** `200 OK`:

```json
{
  "messages": [
    {
      "role": "user",
      "content": "What is Rust?",
      "metadata": null
    },
    {
      "role": "assistant",
      "content": "Rust is a systems programming language...",
      "metadata": null
    }
  ],
  "count": 2
}
```

Message roles: `user`, `assistant`, `tool_use`, `tool_result`.

---

## Workstreams (Auth Required)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/workstreams` | Create workstream |
| GET | `/api/v1/workstreams` | List workstreams |
| GET | `/api/v1/workstreams/{id}` | Get workstream |
| PATCH | `/api/v1/workstreams/{id}` | Update workstream |
| DELETE | `/api/v1/workstreams/{id}` | Delete workstream |
| POST | `/api/v1/workstreams/{id}/messages` | Send message |
| GET | `/api/v1/workstreams/{id}/messages` | List messages |
| GET | `/api/v1/workstreams/{id}/sessions` | List workstream sessions |
| POST | `/api/v1/workstreams/{id}/promote` | Promote content |
| POST | `/api/v1/workstreams/{id}/files/promote` | Promote file from scratch to production |
| POST | `/api/v1/workstreams/{id}/files/export` | Export file to external path |
| POST | `/api/v1/workstreams/{id}/clone` | Clone a git repository into workstream |
| GET | `/api/v1/workstreams/{id}/usage` | Get disk usage statistics |
| POST | `/api/v1/workstreams/{id}/cleanup` | Cleanup inactive scratch data |
| POST | `/api/v1/workstreams/{id}/compress` | Compress session history using LLM |

### POST /api/v1/workstreams

**Request:**

```json
{
  "name": "New Project"
}
```

**Response** `201 Created`: Workstream object.

### POST /api/v1/workstreams/{id}/messages

**Request:**

```json
{
  "message": "Research Rust async patterns"
}
```

**Response** `200 OK`: Message response with agent reply.

### POST /api/v1/workstreams/{id}/promote

Promote scratch content to the production area of the workstream.

**Request:**

```json
{
  "session_id": "ses_abc123",
  "content": "Final research summary..."
}
```

### POST /api/v1/workstreams/{id}/files/promote

**Request:**

```json
{
  "session_id": "ses_abc123",
  "path": "work/report.md"
}
```

### POST /api/v1/workstreams/{id}/files/export

**Request:**

```json
{
  "session_id": "ses_abc123",
  "source_path": "work/report.md",
  "destination": "/home/user/reports/report.md"
}
```

### POST /api/v1/workstreams/{id}/clone

**Request:**

```json
{
  "url": "https://github.com/owner/repo.git",
  "branch": "main"
}
```

### GET /api/v1/workstreams/{id}/usage

**Response** `200 OK`:

```json
{
  "total_bytes": 10485760,
  "session_count": 5,
  "sessions": [
    { "id": "ses_abc", "bytes": 2097152 }
  ]
}
```

### POST /api/v1/workstreams/{id}/cleanup

**Request:**

```json
{
  "older_than_days": 7,
  "dry_run": false
}
```

### POST /api/v1/workstreams/{id}/compress

Compress session history using LLM-based summarization.

**Response** `200 OK`:

```json
{
  "sessions_compressed": 3,
  "tokens_saved": 15000
}
```

---

## Memory (Auth Required)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/memory` | Store a memory |
| GET | `/api/v1/memory/search` | Search memories |
| DELETE | `/api/v1/memory/{id}` | Delete a memory entry |

### POST /api/v1/memory

**Request:**

```json
{
  "content": "The user prefers Rust over Python",
  "content_type": "preference",
  "source": "stated"
}
```

**Response** `201 Created`:

```json
{
  "id": "mem_abc123"
}
```

### GET /api/v1/memory/search

**Query parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `q` | string | yes | Search query |
| `limit` | integer | no | Max results (default 5) |

**Response** `200 OK`:

```json
{
  "results": [
    {
      "id": "mem_abc123",
      "content": "Arawn is written in Rust",
      "confidence": 0.95,
      "source": "stated",
      "created_at": "2025-01-10T08:00:00Z"
    }
  ]
}
```

### DELETE /api/v1/memory/{id}

**Response** `204 No Content`.

---

## Notes (Auth Required)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/notes` | Create note |
| GET | `/api/v1/notes` | List notes |
| GET | `/api/v1/notes/{id}` | Get note |
| PUT | `/api/v1/notes/{id}` | Update note |
| DELETE | `/api/v1/notes/{id}` | Delete note |

### POST /api/v1/notes

**Request:**

```json
{
  "title": "Research Notes",
  "content": "Key findings from today's research...",
  "session_id": "ses_abc123"
}
```

**Response** `201 Created`: Note object with `id`, `title`, `content`, `tags`, `created_at`, `updated_at`.

### GET /api/v1/notes

**Query parameters:** `limit`, `offset`.

**Response** `200 OK`: List of note objects.

### PUT /api/v1/notes/{id}

**Request:**

```json
{
  "title": "Updated Title",
  "content": "Updated content..."
}
```

**Response** `200 OK`: Updated note object.

### DELETE /api/v1/notes/{id}

**Response** `204 No Content`.

---

## Agents (Auth Required)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/agents` | List available agents |
| GET | `/api/v1/agents/{id}` | Get agent details |

### GET /api/v1/agents

**Response** `200 OK`:

```json
{
  "agents": [
    {
      "name": "researcher",
      "description": "Deep research agent",
      "capabilities": { "tools": ["shell", "web_fetch", "file_read"] }
    }
  ]
}
```

### GET /api/v1/agents/{id}

**Response** `200 OK`: Full agent detail including system prompt, model, max iterations, and tool list.

---

## Tasks (Auth Required)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/tasks` | List running tasks |
| GET | `/api/v1/tasks/{id}` | Get task details |
| DELETE | `/api/v1/tasks/{id}` | Cancel a running task |

### DELETE /api/v1/tasks/{id}

**Response** `204 No Content`. Sends cancellation signal to the task.

---

## MCP Servers (Auth Required)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/mcp/servers` | Add MCP server |
| GET | `/api/v1/mcp/servers` | List MCP servers |
| DELETE | `/api/v1/mcp/servers/{name}` | Remove MCP server |
| GET | `/api/v1/mcp/servers/{name}/tools` | List server tools |
| POST | `/api/v1/mcp/servers/{name}/connect` | Connect to server |
| POST | `/api/v1/mcp/servers/{name}/disconnect` | Disconnect from server |

### POST /api/v1/mcp/servers

**Request:**

```json
{
  "name": "sqlite",
  "transport": "stdio",
  "command": "mcp-server-sqlite",
  "args": ["--db", "data.db"]
}
```

**Response** `201 Created`.

### GET /api/v1/mcp/servers

**Response** `200 OK`:

```json
{
  "servers": [
    {
      "name": "sqlite",
      "transport": "stdio",
      "enabled": true,
      "connected": true,
      "tools_count": 5
    }
  ]
}
```

### GET /api/v1/mcp/servers/{name}/tools

**Response** `200 OK`:

```json
{
  "tools": [
    {
      "name": "query",
      "description": "Execute a SQL query",
      "input_schema": { "type": "object", "properties": { "sql": { "type": "string" } } }
    }
  ]
}
```

### POST /api/v1/mcp/servers/{name}/connect

**Response** `200 OK`. Establishes connection to the MCP server.

### POST /api/v1/mcp/servers/{name}/disconnect

**Response** `200 OK`. Gracefully disconnects.

---

## Commands (Auth Required)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/commands` | List available commands |
| POST | `/api/v1/commands/compact` | Run context compaction |
| POST | `/api/v1/commands/compact/stream` | Run context compaction (streaming) |

### POST /api/v1/commands/compact

Compact a session's context by summarizing older turns.

**Request:**

```json
{
  "session_id": "ses_abc123"
}
```

**Response** `200 OK`:

```json
{
  "compacted": true,
  "turns_compacted": 5,
  "tokens_before": 50000,
  "tokens_after": 12000
}
```

### POST /api/v1/commands/compact/stream

Same request as above. Returns SSE stream with progress events.

---

## Config (Auth Required)

### GET /api/v1/config

Returns the server's active configuration (sanitized, no secrets).

**Response** `200 OK`:

```json
{
  "features": {
    "memory": true,
    "plugins": true,
    "mcp": true,
    "pipeline": false
  },
  "limits": {
    "max_body_size": 10485760,
    "max_ws_message_size": 1048576,
    "api_rpm": 120
  }
}
```

---

## Logs (Auth Required)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/logs` | Get log entries |
| GET | `/api/v1/logs/files` | List available log files |

### GET /api/v1/logs

**Query parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `lines` | integer | 25 | Number of lines to return |
| `file` | string | latest | Log file name |

**Response** `200 OK`:

```json
{
  "file": "2025-01-10.log",
  "count": 25,
  "entries": [
    { "line": "2025-01-10T08:00:00Z INFO server started on 127.0.0.1:8080" }
  ]
}
```

### GET /api/v1/logs/files

**Response** `200 OK`:

```json
{
  "files": [
    { "name": "2025-01-10.log", "size": 102400 }
  ]
}
```
