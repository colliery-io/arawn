# WebSocket Protocol Reference

Complete reference for the Arawn WebSocket protocol. The WebSocket transport provides real-time bidirectional communication between clients and the server.

## Connection

### Endpoint

```
ws://localhost:8080/ws
ws://localhost:8080/ws?workstream=ws_abc123
```

Upgrade via a standard HTTP `GET` request with `Upgrade: websocket` headers.

### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `workstream` | string | Optional workstream ID to associate with the connection |

### Security

| Setting | Value | Description |
|---------|-------|-------------|
| Origin validation | Configurable | Checked against `[server] ws_allowed_origins`. Empty list allows all origins. |
| Connection rate limit | 30/min/IP | Maximum WebSocket connections per minute per IP address |
| Max message size | 1 MB | Messages exceeding this size are rejected |
| Idle timeout | 5 min | Connections without activity are closed. Send `ping` to keep alive. |
| Max body size | 10 MB | HTTP upgrade request body limit |

---

## Authentication Flow

Authentication is performed via the WebSocket message channel, not HTTP headers.

1. Client connects via `GET /ws` (upgrade).
2. Client sends an `auth` message with a bearer token.
3. Server responds with `auth_result`.
4. If `success: true`, the connection is authenticated and subsequent messages are accepted.
5. If `success: false`, the server closes the connection.

Unauthenticated connections that send `chat` or other protected messages receive an error response.

```
Client                          Server
  |                               |
  |--- WebSocket Upgrade -------->|
  |<-- 101 Switching Protocols ---|
  |                               |
  |--- auth {token} ------------->|
  |<-- auth_result {success} -----|
  |                               |
  |--- chat {message} ----------->|
  |<-- session_created -----------|
  |<-- chat_chunk --------------->|
  |<-- chat_chunk {done: true} ---|
  |                               |
```

---

## Message Format

All messages are JSON objects with a `type` field. Messages use `snake_case` for the type discriminator and all field names.

---

## Client-to-Server Messages

### auth

Authenticate the connection.

```json
{
  "type": "auth",
  "token": "your-bearer-token"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `token` | string | yes | Bearer token for authentication |

### chat

Send a chat message to the agent.

```json
{
  "type": "chat",
  "message": "Explain async in Rust",
  "session_id": "ses_abc123",
  "workstream_id": "ws_def456"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `message` | string | yes | The user message content |
| `session_id` | string | no | Existing session to continue. If omitted, a new session is created. |
| `workstream_id` | string | no | Workstream context. If omitted, uses the "scratch" workstream. |

### subscribe

Subscribe to updates for a session. Allows observing sessions owned by other connections.

```json
{
  "type": "subscribe",
  "session_id": "ses_abc123",
  "reconnect_token": "tok_xyz789"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `session_id` | string | yes | Session ID to subscribe to |
| `reconnect_token` | string | no | Token from a previous `subscribe_ack` for reclaiming ownership after disconnect |

### unsubscribe

Stop receiving updates for a session.

```json
{
  "type": "unsubscribe",
  "session_id": "ses_abc123"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `session_id` | string | yes | Session ID to unsubscribe from |

### cancel

Cancel the current operation on a session.

```json
{
  "type": "cancel",
  "session_id": "ses_abc123"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `session_id` | string | yes | Session ID whose operation to cancel |

### ping

Keep-alive message. The server responds with `pong`.

```json
{
  "type": "ping"
}
```

### command

Execute a server command.

```json
{
  "type": "command",
  "command": "compact",
  "args": {
    "session_id": "ses_abc123",
    "force": true
  }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `command` | string | yes | Command name (e.g., `"compact"`) |
| `args` | object | no | Command arguments as a JSON object. Defaults to `null`. |

---

## Server-to-Client Messages

### auth_result

Response to an `auth` message.

```json
{
  "type": "auth_result",
  "success": true
}
```

```json
{
  "type": "auth_result",
  "success": false,
  "error": "Invalid token"
}
```

| Field | Type | Always Present | Description |
|-------|------|----------------|-------------|
| `success` | bool | yes | Whether authentication succeeded |
| `error` | string | no | Error message (only on failure) |

### session_created

Confirms a session was created or resumed after a `chat` message.

```json
{
  "type": "session_created",
  "session_id": "ses_abc123"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `session_id` | string | The session ID |

### chat_chunk

Streaming text from the agent response.

```json
{
  "type": "chat_chunk",
  "session_id": "ses_abc123",
  "chunk": "Rust's ownership model ",
  "done": false
}
```

| Field | Type | Description |
|-------|------|-------------|
| `session_id` | string | Session ID this chunk belongs to |
| `chunk` | string | Text content |
| `done` | bool | `true` on the final chunk of the response |

### tool_start

A tool execution has started.

```json
{
  "type": "tool_start",
  "session_id": "ses_abc123",
  "tool_id": "tc_001",
  "tool_name": "shell"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `session_id` | string | Session ID |
| `tool_id` | string | Unique tool call ID |
| `tool_name` | string | Name of the tool being executed |

### tool_output

Streaming output from a running tool.

```json
{
  "type": "tool_output",
  "session_id": "ses_abc123",
  "tool_id": "tc_001",
  "content": "cargo build output..."
}
```

| Field | Type | Description |
|-------|------|-------------|
| `session_id` | string | Session ID |
| `tool_id` | string | Tool call ID |
| `content` | string | Output content chunk |

### tool_end

A tool execution has completed.

```json
{
  "type": "tool_end",
  "session_id": "ses_abc123",
  "tool_id": "tc_001",
  "success": true
}
```

| Field | Type | Description |
|-------|------|-------------|
| `session_id` | string | Session ID |
| `tool_id` | string | Tool call ID |
| `success` | bool | Whether the tool execution succeeded |

### subscribe_ack

Acknowledgment of a `subscribe` message.

```json
{
  "type": "subscribe_ack",
  "session_id": "ses_abc123",
  "owner": true,
  "reconnect_token": "tok_xyz789"
}
```

| Field | Type | Always Present | Description |
|-------|------|----------------|-------------|
| `session_id` | string | yes | Session ID subscribed to |
| `owner` | bool | yes | Whether this connection is the session owner (can send `chat`) |
| `reconnect_token` | string | no | Token for reclaiming ownership after disconnect. Only present when `owner: true`. |

### command_progress

Progress update for a running command.

```json
{
  "type": "command_progress",
  "command": "compact",
  "message": "Summarizing turns 1-5...",
  "percent": 50
}
```

| Field | Type | Always Present | Description |
|-------|------|----------------|-------------|
| `command` | string | yes | Command name |
| `message` | string | yes | Human-readable progress message |
| `percent` | u8 | no | Progress percentage (0-100) |

### command_result

Final result of a command execution.

```json
{
  "type": "command_result",
  "command": "compact",
  "success": true,
  "result": {
    "compacted": true,
    "turns_compacted": 5
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `command` | string | Command name |
| `success` | bool | Whether the command succeeded |
| `result` | object | Result data (on success) or `{"error": "..."}` (on failure) |

### context_info

Context window usage information for a session. Sent periodically during active conversations.

```json
{
  "type": "context_info",
  "session_id": "ses_abc123",
  "current_tokens": 80000,
  "max_tokens": 100000,
  "percent": 80,
  "status": "warning"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `session_id` | string | Session ID |
| `current_tokens` | integer | Current estimated token count |
| `max_tokens` | integer | Maximum allowed tokens |
| `percent` | u8 | Usage as percentage (0-100) |
| `status` | string | `"ok"`, `"warning"`, or `"critical"` |

Status thresholds:

| Status | Condition | Description |
|--------|-----------|-------------|
| `ok` | < 70% | Normal operation |
| `warning` | 70% - 89% | Approaching context limit; consider compaction |
| `critical` | >= 90% | Near context limit; compaction strongly recommended |

### fs_change

Filesystem change notification for a workstream.

```json
{
  "type": "fs_change",
  "workstream": "my-project",
  "path": "production/report.md",
  "action": "modified",
  "timestamp": "2025-01-10T08:00:00Z"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `workstream` | string | Workstream where the change occurred |
| `path` | string | Relative path within the workstream |
| `action` | string | `"created"`, `"modified"`, or `"deleted"` |
| `timestamp` | string | ISO 8601 timestamp |

### disk_pressure

Disk usage alert.

```json
{
  "type": "disk_pressure",
  "level": "warning",
  "scope": "my-workstream",
  "usage_mb": 1800.0,
  "limit_mb": 2048.0,
  "timestamp": "2025-01-10T08:00:00Z"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `level` | string | `"ok"`, `"warning"`, or `"critical"` |
| `scope` | string | `"total"` or a workstream ID |
| `usage_mb` | float | Current usage in megabytes |
| `limit_mb` | float | Configured limit in megabytes |
| `timestamp` | string | ISO 8601 timestamp |

### error

Error notification. Sent when processing a client message fails.

```json
{
  "type": "error",
  "code": "unauthorized",
  "message": "Authentication required"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `code` | string | Machine-readable error code |
| `message` | string | Human-readable error description |

Error codes: `unauthorized`, `invalid_message`, `session_not_found`, `rate_limited`, `internal_error`.

### pong

Response to a `ping` message.

```json
{
  "type": "pong"
}
```

---

## Session Ownership

WebSocket connections can subscribe to sessions. Only one connection at a time is the session **owner**, meaning it can send `chat` messages to that session.

### Ownership Rules

1. The connection that creates a session (via `chat` with no `session_id`) is automatically the owner.
2. When subscribing to an existing session, the `subscribe_ack` message indicates `owner: true` or `owner: false`.
3. Non-owner connections receive all streaming events but cannot send `chat` messages to that session.

### Reconnection

When an owner disconnects, there is a **30-second grace period** during which the session remains reserved. To reclaim ownership:

1. Reconnect and authenticate.
2. Send a `subscribe` message with the `reconnect_token` received from the original `subscribe_ack`.
3. If within the grace period, the server grants ownership and responds with `owner: true`.
4. If the grace period has expired, another connection may have claimed ownership.

---

## Complete Session Lifecycle

```
Client                              Server
  |                                   |
  |--- auth {token} ----------------->|
  |<-- auth_result {success: true} ---|
  |                                   |
  |--- chat {message: "Hi"} -------->|
  |<-- session_created {id: "s1"} ---|
  |<-- chat_chunk {chunk: "Hello"} --|
  |<-- chat_chunk {done: true} ------|
  |<-- context_info {percent: 5} ----|
  |                                   |
  |--- subscribe {session_id: "s1"} ->|
  |<-- subscribe_ack {owner: true,    |
  |      reconnect_token: "tok123"} --|
  |                                   |
  |--- chat {session_id: "s1",       |
  |     message: "Tell me more"} ---->|
  |<-- tool_start {tool: "shell"} ---|
  |<-- tool_output {content: "..."} -|
  |<-- tool_end {success: true} -----|
  |<-- chat_chunk {chunk: "..."} ----|
  |<-- chat_chunk {done: true} ------|
  |<-- context_info {percent: 15} ---|
  |                                   |
  |--- cancel {session_id: "s1"} --->|
  |<-- chat_chunk {done: true} ------|
  |                                   |
  |--- ping ------------------------->|
  |<-- pong --------------------------|
  |                                   |
  |--- unsubscribe {id: "s1"} ------>|
  |                                   |
```
