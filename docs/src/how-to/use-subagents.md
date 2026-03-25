# Use Subagents and Delegation

This guide explains how to use subagents to break complex tasks into specialized subtasks. You will learn the `delegate` and `explore` tools, how to configure agents, and when to choose delegation versus exploration.

## Prerequisites

- Arawn running with at least one plugin that defines agents, or the built-in exploration agent
- Familiarity with the [Plugin System](../explanation/plugins-and-hooks.md)

## List available agents

Check which agents are available in your installation:

```bash
# List all agents
arawn agent list

# Filter by plugin
arawn agent list --plugin my-plugin

# Show details for a specific agent
arawn agent info researcher
```

The output includes the agent name, description, allowed tools, and which plugin provides it.

## Delegate a task

The `delegate` tool spawns a named subagent with a focused task. Use it when you need a specialist:

```json
{
  "name": "delegate",
  "parameters": {
    "agent": "researcher",
    "task": "Find the top 5 Rust crates for async HTTP clients, comparing features and download counts",
    "context": "User is building a REST API client library",
    "background": false,
    "max_turns": 10
  }
}
```

### Parameters

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `agent` | Yes | string | Subagent name (must match a defined agent) |
| `task` | Yes | string | What the subagent should accomplish |
| `context` | No | string | Background from the parent conversation (truncated at 4000 chars) |
| `background` | No | bool | Run asynchronously (default: `false`) |
| `max_turns` | No | integer | Override the agent's default iteration limit |

### Blocking delegation

By default, delegation is blocking. The parent agent waits for the subagent to finish and receives the result inline:

```
## Result from 'researcher'

Here are the top 5 Rust async HTTP crates:
1. reqwest — 45M downloads, full-featured...
...
```

Use blocking mode for tasks where the parent needs the result before continuing.

### Background delegation

Set `background: true` to spawn the subagent without waiting:

```json
{
  "name": "delegate",
  "parameters": {
    "agent": "reviewer",
    "task": "Review the authentication module for security issues",
    "background": true
  }
}
```

The parent receives an immediate confirmation:

```
Delegated to 'reviewer' in background. You'll be notified when complete.
```

Use background mode for tasks that are independent of the main conversation flow, or when you want to run multiple agents in parallel.

## Explore with the RLM agent

The `explore` tool spawns an isolated Recursive Language Model (RLM) sub-agent for open-ended research. Unlike `delegate`, it does not require a named agent definition:

```json
{
  "name": "explore",
  "parameters": {
    "query": "How does the authentication middleware handle token refresh?"
  }
}
```

### How exploration works

1. The RLM agent starts with your query and a set of read-only tools (`file_read`, `grep`, `glob`, `web_search`, etc.).
2. It iterates: reads files, searches code, gathers information.
3. When context grows too large, it compacts (summarizes) what it has learned and continues.
4. After reaching a conclusion or hitting limits, it returns a summary with metadata.

### Exploration output

The result includes a summary and a metadata footer:

```
The authentication middleware uses a two-layer approach:
1. JWT validation in the auth_middleware function...
2. Token refresh is handled by the OAuth proxy...

---
Exploration: 8 iterations, 24530 tokens (18200in/6330out), 2 compactions
```

### When to use explore vs delegate

| Scenario | Use |
|----------|-----|
| Answering a research question about the codebase | `explore` |
| Summarizing a long document | `explore` |
| Running a defined workflow (review, deploy, test) | `delegate` |
| Task requiring write access (file edits, shell commands) | `delegate` |
| Quick factual lookup across many files | `explore` |
| Task needing a specific system prompt and tool set | `delegate` |

Key difference: `explore` is read-only and self-managing. `delegate` gives full control over the agent's identity, tools, and permissions.

## Configure RLM exploration

Add an `[rlm]` section to your config file to tune exploration behavior:

```toml
[rlm]
model = "claude-sonnet-4-20250514"       # Model for exploration (default: inherit from backend)
max_turns = 50                            # Maximum agent turns (safety valve)
max_context_tokens = 50000                # Token threshold for triggering compaction
compaction_threshold = 0.7                # Fraction of max_context_tokens to trigger compaction
max_compactions = 10                      # Maximum compaction cycles
max_total_tokens = 500000                 # Cumulative token budget for the entire exploration
compaction_model = "claude-haiku-4-20250514"  # Cheaper model for compaction summaries
```

| Setting | Default | Description |
|---------|---------|-------------|
| `model` | (inherited) | LLM model for the exploration agent |
| `max_turns` | 50 | Hard limit on agent iterations |
| `max_context_tokens` | 50,000 | Context size threshold for compaction |
| `compaction_threshold` | 0.7 | Trigger compaction at 70% of `max_context_tokens` |
| `max_compactions` | 10 | Stop after this many compaction cycles |
| `max_total_tokens` | (none) | Total token budget across all iterations |
| `compaction_model` | (none) | Separate model for compaction (saves cost) |

## Configure delegation

The `[delegation]` section controls how subagent results are handled:

```toml
[delegation]
max_result_len = 8000    # Truncate results longer than this (characters)

[delegation.compaction]
enabled = true           # Use LLM compaction instead of truncation
threshold = 8000         # Minimum length to trigger compaction
backend = "default"      # LLM profile for compaction
model = "gpt-4o-mini"   # Model for compaction
target_len = 4000        # Target length after compaction
```

When `compaction.enabled` is `true`, long results are summarized by an LLM call instead of being hard-truncated. This preserves more useful information at the cost of an additional API call.

When compaction is disabled, results longer than `max_result_len` are truncated, preserving the first 65% and last 35% with a truncation notice in between.

## Define a subagent in a plugin

Subagents are Markdown files in a plugin's `agents/` directory:

```
my-plugin/
  agents/
    researcher.md
    reviewer.md
  plugin.toml
```

Each file uses YAML frontmatter followed by the system prompt:

```markdown
---
name: researcher
description: Web research specialist
model: sonnet
tools: ["web_fetch", "web_search", "think"]
max_iterations: 10
---

You are a research assistant specialized in finding accurate information.

## Your Process
1. Understand the research question
2. Search for relevant sources
3. Synthesize findings
4. Cite your sources

Always verify information across multiple sources when possible.
```

### Frontmatter fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Unique identifier used in `delegate` calls |
| `description` | Yes | Human-readable description shown in `arawn agent list` |
| `model` | No | Model override (references an LLM profile name) |
| `tools` | No | Allowed tools from the parent's registry |
| `max_iterations` | No | Maximum turns before the agent stops |

The `tools` list is a security boundary. The subagent can only access tools listed here, even though the parent may have more tools available.

## Configure per-agent LLM profiles

Use the `[agent.<name>]` section to give specific agents their own LLM configuration, system prompt, and iteration limits:

```toml
[llm]
backend = "groq"
model = "llama-3.3-70b-versatile"

[llm_profiles.claude]
backend = "anthropic"
model = "claude-sonnet-4-20250514"

[agent.default]
llm = "claude"
max_iterations = 20

[agent.summarizer]
llm = "claude"
system_prompt = "You are a concise summarizer. Always respond in bullet points."
max_iterations = 5
max_tokens = 2000
```

Resolution order for an agent named `summarizer`:
1. `[agent.summarizer]` settings (if defined)
2. `[agent.default]` settings (fallback)
3. Global `[llm]` section

## Handle unknown agents

If you delegate to an agent name that does not exist, you get an error with a list of available agents:

```
Unknown agent 'analyser'. Available agents: researcher, reviewer
```

Verify agent availability with `arawn agent list` before delegating.

## Best practices

1. **Design focused agents.** Each agent should do one thing well. A "researcher" should not also deploy code.
2. **Grant minimal tools.** Only include tools the agent actually needs. A code reviewer needs `file_read` and `grep`, not `shell`.
3. **Provide clear context.** The `context` parameter helps subagents understand the situation without re-discovering it.
4. **Set iteration limits.** Prevent runaway execution with `max_turns` or `max_iterations`.
5. **Use explore for open-ended research.** If you do not know what agent to use, `explore` is the right default for investigation tasks.
6. **Use compaction for long results.** Enable `[delegation.compaction]` if your agents produce verbose output.
