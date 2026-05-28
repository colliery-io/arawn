# The agent loop

*Explanation. How a turn flows from your message to the agent's response.*

Every conversational turn in arawn moves through the same loop: prompt assembly → LLM call → tool call(s) → permission check → execution → response. The interesting question isn't *what* the loop does (that's obvious) but *why* each step exists. This page is the why.

## The shape

```
You type a message.
   │
   ▼
1. System prompt assembly
   - identity prompt (always `assistant` — persona is not per-lens)
   - global memory entries that match the user's text
   - the live tool registry (built-ins + integration tools + plugin tools + MCP tools)
   - any `arawn.md` directives in the data_dir
   │
   ▼
2. LLM call
   The provider returns: text fragments + zero-or-more tool calls.
   │
   ▼
3. For each tool call:
   a. Permission check (rule → mode fallback). Outcome: allow / ask / deny.
   b. If ask, user is prompted; allow_once / allow_always / deny.
   c. If allowed, dispatch:
      - shell tools → OS sandbox (sandbox-exec / bubblewrap)
      - file tools  → lens workspace path enforcement
      - integration → cached creds + retry/refresh
   d. Result returns to the loop as a tool result.
   │
   ▼
4. If any tool calls happened, loop to step 2.
   Otherwise, the response is done.
```

## Why "assemble" the system prompt fresh each turn

The system prompt isn't static. Several pieces change between turns:

- **Global memory entries** that match the current user message get injected as context. If you ask "what did we decide about Postgres?", the memory loader pre-fetches relevant entities so the LLM sees them in-context. (The chat persona is always `assistant`; lenses do not change it — see [identity by lens](./identity-by-lens.md).)
- **Tools** change as integrations connect/disconnect, plugins load/unload, MCP servers come and go. Hot-reload means the registry can change mid-session.
- **`arawn.md` files** in the data directory carry persistent behavioral directives. They're injected into every turn so the agent stays consistent across sessions.

The cost: a few hundred bytes of redundant assembly per turn. The win: the agent's view of the world is always current.

## Why a tool-call loop instead of single-shot

A modern LLM is excellent at one-shot text generation, but agentic tasks need *iteration*. "What's in my inbox?" requires reading the inbox before answering. "Find every TODO in this repo" requires grepping, then summarizing.

The loop is the answer: the LLM emits a tool call, the engine runs it, the result feeds back, and the LLM decides what to do next. Two consequences:

1. **Plans aren't pre-committed.** The agent can investigate, hit a wall, change tactics. A single-shot prompt couldn't — it would have to predict the whole workflow up front.
2. **Tool result size matters.** Each iteration consumes context. The compactor (see below) and the `max_result_size` engine setting bound how much tool output can flow back.

The `max_iterations` engine setting caps the loop at 20 by default. Hit the cap → the engine forces a final response.

## Why permission checks instead of just running

arawn lets an LLM run shell commands and edit files on your machine. That's a lot of trust. Permission checks let you scope that trust — auto-allow safe things, ask about risky things, deny things the agent should never do regardless of mode.

The model is documented in [permission model](./permission-model.md). What matters here:

- Every tool call goes through the check. Including `Read`, `Glob`, `Grep` (which auto-allow in `default` mode).
- A failed check returns a tool result like any other. The LLM sees "denied" and reacts — it can ask the user, change approach, or give up cleanly.
- Modes give you global posture. Rules give you fine control. Together they cover the spectrum from "ask me every keystroke" to "go nuts in this sandbox VM."

## Why a sandbox under the shell tool

Permission rules say what the agent *should* do. The sandbox enforces what the agent *can* do at the OS level. They compose:

- Rule says allow `shell(rm -rf node_modules)`: the call goes through.
- Sandbox sees the command tries to read `~/.ssh/id_rsa` (none of the rm code does this, but say it did): the syscall fails.

Without the sandbox, a single misconfigured permission rule turns the agent into a credential-stealing risk. The sandbox makes that risk bounded. See [shell sandbox reference](../reference/shell-sandbox.md) for the enforcement details.

## Why context compaction

LLM context windows are finite. A long conversation that's accumulated dozens of tool results will hit the wall. arawn's compactor (`crates/arawn-engine/src/compactor.rs`) watches the running token estimate; when it crosses `compaction_threshold` (default 0.85), it:

1. Keeps `keep_recent` (default 6) most-recent messages verbatim.
2. Asks a cheaper LLM (`[compactor].llm`) to summarize everything before that.
3. Replaces the older messages with the summary.

The conversation continues. You don't see the compaction unless you look at the engine log. Memory entries are NOT touched — they're independent of conversational context.

## Why tools are bounded

Three caps protect the loop from runaway behavior:

| Cap | Default | What it bounds |
|---|---|---|
| `[engine].max_iterations` | 20 | Tool-use iterations per turn. |
| `[engine].max_result_size` | 50000 (bytes) | Max bytes from one tool result that feed back to the LLM. |
| `[engine].tool_timeout_secs` | 120 | Wall-clock timeout per tool call. |

A grep that returns 5 MB gets truncated. A shell that loops forever gets killed. A conversation that's a turn long on a tight loop gets force-completed. None of these are normal — they're guard rails for when something goes wrong.

## What's NOT in the loop

- **Watchers and ceremonies.** Those run on cron via [cloacina](https://github.com/colliery-io/cloacina) workflows. They have their own short-lived agent loops; the conversational loop doesn't see them directly.
- **Background sub-agents.** When the agent spawns a sub-agent with `run_in_background: true`, the sub-agent runs in parallel; the parent continues. See [sub-agents reference](../reference/sub-agents.md).
- **The extractor.** When new projection rows land, the extractor fires its own CoT chain to build palace entities. Not in the conversational loop.

The conversational loop is the *interactive surface* — the part you experience as a chat. The watching, checking, and extracting are happening continuously around it.

## Related

- [Permission model](./permission-model.md) — why deny > allow > ask.
- [Three-layer data model](./three-layer-data-model.md) — what the agent reads.
- [Sub-agents reference](../reference/sub-agents.md) — delegation and parallelism.
- [Agent tools reference](../reference/agent-tools.md) — what's in the tool registry.
- [Permissions reference](../reference/permissions.md) — rule syntax + audit log.
