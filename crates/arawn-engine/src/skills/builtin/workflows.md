---
name: workflows
description: "Use when the user asks to create, manage, or debug scheduled workflows. Covers workflow creation patterns, decision task HTTP callbacks, cron scheduling, and the full scaffold-compile-install lifecycle."
user_invocable: true
---

# Workflow Authoring Guide

Arawn workflows are compiled Rust crates that run as scheduled DAG pipelines via the embedded cloacina engine. The agent authors them during conversation using the `workflow_create` tool.

## Tool Reference

| Tool | Purpose |
|------|---------|
| `workflow_create` | Scaffold, compile, and install a new workflow |
| `workflow_list` | List installed workflows and cron schedules |
| `workflow_delete` | Remove an installed workflow |
| `workflow_status` | Check recent execution history (pass/fail/running) |

## Task Types

Workflows are heterogeneous DAGs mixing three task flavors:

### Data tasks — pure code, no LLM
Fetch APIs, query databases, transform data. Fast, deterministic, retryable.

```rust
ctx.insert("data", serde_json::json!({"items": results}))?;
Ok(())
```

### Decision tasks — agent-powered reasoning
Call back to the arawn server's decision endpoint for multi-turn agent sessions with workstream context.

```rust
let client = reqwest::Client::new();
let upstream = ctx.get("data").cloned().unwrap_or(serde_json::json!(null));
let resp = client.post("http://127.0.0.1:3100/api/decision")
    .json(&serde_json::json!({
        "prompt": "Triage these items and decide which need attention",
        "workstream": "my-project",
        "upstream_data": upstream,
    }))
    .send()
    .await
    .map_err(|e| TaskError::ExecutionFailed {
        message: e.to_string(),
        task_id: "decide".into(),
        timestamp: chrono::Utc::now(),
    })?;
let result: serde_json::Value = resp.json().await.map_err(|e| TaskError::ExecutionFailed {
    message: e.to_string(),
    task_id: "decide".into(),
    timestamp: chrono::Utc::now(),
})?;
ctx.insert("decision", result)?;
Ok(())
```

### Action tasks — execute decisions
Write files, send notifications, schedule follow-ups based on upstream decision results.

```rust
let decision = ctx.get("decision").cloned().unwrap_or_default();
// Act on the decision...
Ok(())
```

## Creating a Workflow

When the user describes a recurring automation, use `workflow_create` with:

1. **name** — kebab-case identifier (e.g., `github-triage`)
2. **description** — what it does
3. **tasks** — array of `{id, dependencies, body}` forming the DAG
4. **cron** — optional cron expression (e.g., `0 8 * * 1-5` for weekday mornings)

### Example: Daily summary workflow

```json
{
  "name": "daily-summary",
  "description": "Fetch updates and produce a daily summary",
  "tasks": [
    {
      "id": "fetch",
      "dependencies": [],
      "body": "let data = reqwest::get(\"https://api.example.com/updates\").await.map_err(|e| TaskError::ExecutionFailed { message: e.to_string(), task_id: \"fetch\".into(), timestamp: chrono::Utc::now() })?.json::<serde_json::Value>().await.map_err(|e| TaskError::ExecutionFailed { message: e.to_string(), task_id: \"fetch\".into(), timestamp: chrono::Utc::now() })?;\nctx.insert(\"updates\", data)?;\nOk(())"
    },
    {
      "id": "summarize",
      "dependencies": ["fetch"],
      "body": "let client = reqwest::Client::new();\nlet upstream = ctx.get(\"updates\").cloned().unwrap_or(serde_json::json!(null));\nlet resp = client.post(\"http://127.0.0.1:3100/api/decision\").json(&serde_json::json!({\"prompt\": \"Summarize these updates into a concise daily briefing\", \"workstream\": \"scratch\", \"upstream_data\": upstream})).send().await.map_err(|e| TaskError::ExecutionFailed { message: e.to_string(), task_id: \"summarize\".into(), timestamp: chrono::Utc::now() })?;\nlet result: serde_json::Value = resp.json().await.map_err(|e| TaskError::ExecutionFailed { message: e.to_string(), task_id: \"summarize\".into(), timestamp: chrono::Utc::now() })?;\nctx.insert(\"summary\", result)?;\nOk(())"
    }
  ],
  "cron": "0 8 * * 1-5"
}
```

## Important Notes

- **First build is slow** — the scaffold crate downloads dependencies on first compile. Subsequent builds reuse the cargo cache.
- **Task bodies are raw Rust** — they must be valid async Rust that compiles. If compilation fails, the error is returned and you should fix the code and retry.
- **Decision tasks need reqwest** — the scaffold Cargo.toml already includes `reqwest`. For other deps, the user needs to extend the template.
- **Cron runs in UTC by default** — use `cron_timezone` to override.
- **Hot-loading** — the cloacina reconciler watches `~/.arawn/workflows/` and picks up new packages automatically. No server restart needed.
- **Debugging** — use `workflow_status` to check execution history. Failed tasks show error messages.
