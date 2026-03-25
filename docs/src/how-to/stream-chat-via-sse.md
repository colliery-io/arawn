# Stream Chat via SSE

This guide shows how to send messages to the Arawn server and receive streamed responses using Server-Sent Events (SSE). You will learn the endpoint contract, event types, and how to consume the stream from curl, Python, and Rust.

## Prerequisites

- Arawn server running (`arawn start`)
- A valid API token (see [Secure Your Deployment](secure-your-deployment.md) or [Configuration Reference](../reference/configuration.md))
- The server URL (default: `http://127.0.0.1:8080`)

## The streaming endpoint

```
POST /api/v1/chat/stream
Content-Type: application/json
Authorization: Bearer <token>
```

### Request body

```json
{
  "session_id": "optional-uuid",
  "message": "Your question or instruction"
}
```

| Field | Required | Type | Description |
|-------|----------|------|-------------|
| `message` | Yes | string | The user message (max 100KB) |
| `session_id` | No | string (UUID) | Existing session to continue. Omit to create a new session. |

### Response

The response has content type `text/event-stream` and delivers a sequence of Server-Sent Events. Each event has a named type and a JSON data payload.

## Event types

Events arrive in this general order: `session` first, then interleaved `text`/`tool_*` events, and finally `done` or `error`.

| Event | Key Fields | Description |
|-------|-----------|-------------|
| `session` | `session_id` | Sent first. Contains the session ID. |
| `text` | `content` | A chunk of streamed text. Concatenate to build the full response. |
| `tool_start` | `id`, `name` | A tool execution has started. |
| `tool_output` | `id`, `content` | Output from a running tool. |
| `tool_end` | `id`, `success` | A tool execution has completed. |
| `done` | `iterations` | The agent finished its response. |
| `error` | `message` | An error occurred. The stream ends after this event. |

For detailed event schemas, see the [REST API Reference](../reference/api.md).

## Test with curl

```bash
curl -N -X POST http://127.0.0.1:8080/api/v1/chat/stream \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"message": "List the files in the current directory"}'
```

The `-N` flag disables output buffering so events print as they arrive. You will see raw SSE output:

```
event: session
data: {"session_id":"a1b2c3d4-..."}

event: text
data: {"content":"I'll list the files"}

event: tool_start
data: {"id":"tc_001","name":"shell"}

event: tool_output
data: {"id":"tc_001","content":"src/\nCargo.toml\nREADME.md"}

event: tool_end
data: {"id":"tc_001","success":true,"content":""}

event: text
data: {"content":"Here are the files in the current directory:\n- src/\n- Cargo.toml\n- README.md"}

event: done
data: {"iterations":1}
```

To continue a conversation, pass the `session_id` from the first response:

```bash
curl -N -X POST http://127.0.0.1:8080/api/v1/chat/stream \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"session_id": "a1b2c3d4-...", "message": "Now show me Cargo.toml"}'
```

## Consume from Python

Use the `sseclient-py` library to parse the event stream:

```python
import json
import requests
import sseclient

url = "http://127.0.0.1:8080/api/v1/chat/stream"
headers = {
    "Authorization": "Bearer YOUR_TOKEN",
    "Content-Type": "application/json",
}
payload = {"message": "Explain the project structure"}

response = requests.post(url, json=payload, headers=headers, stream=True)
response.raise_for_status()

client = sseclient.SSEClient(response)
session_id = None
full_text = ""

for event in client.events():
    data = json.loads(event.data)

    if event.event == "session":
        session_id = data["session_id"]
        print(f"Session: {session_id}")

    elif event.event == "text":
        full_text += data["content"]
        print(data["content"], end="", flush=True)

    elif event.event == "tool_start":
        print(f"\n[Tool: {data['name']}]", flush=True)

    elif event.event == "tool_output":
        print(f"  {data['content'][:200]}", flush=True)

    elif event.event == "tool_end":
        status = "ok" if data["success"] else "failed"
        print(f"[Tool {status}]", flush=True)

    elif event.event == "done":
        print(f"\n--- Done ({data['iterations']} iterations) ---")

    elif event.event == "error":
        print(f"\nError: {data['message']}")
        break

print(f"\nFull response length: {len(full_text)} chars")
```

Install the dependency: `pip install sseclient-py requests`

## Consume from Rust with arawn-client

The `arawn-client` crate provides a typed streaming API:

```rust
use arawn_client::{ArawnClient, ChatRequest};
use futures::StreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = ArawnClient::builder()
        .base_url("http://127.0.0.1:8080")
        .token("YOUR_TOKEN")
        .build()?;

    let request = ChatRequest::new("Explain the project structure");
    let mut stream = client.chat().stream(request).await?;

    while let Some(event) = stream.next().await {
        match event? {
            StreamEvent::SessionStart { session_id, .. } => {
                println!("Session: {session_id}");
            }
            StreamEvent::Content { text } => {
                print!("{text}");
            }
            StreamEvent::ToolStart { tool_name, .. } => {
                println!("\n[Tool: {tool_name}]");
            }
            StreamEvent::ToolOutput { content, .. } => {
                println!("  {}", &content[..content.len().min(200)]);
            }
            StreamEvent::ToolEnd { success, .. } => {
                println!("[Tool {}]", if success { "ok" } else { "failed" });
            }
            StreamEvent::Done { response, .. } => {
                println!("\n--- Done ---");
            }
            StreamEvent::Error { message } => {
                eprintln!("Error: {message}");
                break;
            }
        }
    }

    Ok(())
}
```

Add the dependency: `cargo add arawn-client`

## Handle errors

### Authentication errors (401)

If the token is missing or invalid, the server returns HTTP 401 before the SSE stream starts:

```bash
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://127.0.0.1:8080/api/v1/chat/stream \
  -H "Content-Type: application/json" \
  -d '{"message": "hello"}'
# 401
```

Fix: include a valid `Authorization: Bearer <token>` header.

### Rate limiting (429)

When rate limiting is enabled (`[server] rate_limiting = true`), exceeding the request quota returns HTTP 429 with a `Retry-After` header:

```
HTTP/1.1 429 Too Many Requests
Retry-After: 12
```

Wait the indicated number of seconds before retrying.

### Message too large (400)

Messages exceeding 100KB are rejected with HTTP 400:

```json
{"error": "Message too large: 150000 bytes (max 102400 bytes)"}
```

### Connection drops

If the connection drops mid-stream (network issue, server restart), the SSE stream ends without a `done` event. Your client should:

1. Detect the unexpected end of stream.
2. Retry the request with the same `session_id` to continue the conversation.
3. Use exponential backoff for retries.

## Use the synchronous alternative

If you do not need streaming, use the synchronous endpoint instead. It waits for the full response and returns it as a single JSON object:

```
POST /api/v1/chat
Content-Type: application/json
Authorization: Bearer <token>
```

```bash
curl -X POST http://127.0.0.1:8080/api/v1/chat \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"message": "What time is it?"}'
```

Response:

```json
{
  "session_id": "a1b2c3d4-...",
  "response": "I don't have access to the current time...",
  "tool_calls": [],
  "truncated": false,
  "usage": {
    "input_tokens": 150,
    "output_tokens": 42
  }
}
```

The synchronous endpoint is simpler to integrate but provides no incremental feedback. For interactive UIs, prefer the streaming endpoint.
