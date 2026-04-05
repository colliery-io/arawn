---
id: context-compaction-token-aware
level: initiative
title: "Context Compaction — token-aware conversation summarization"
short_code: "ARAWN-I-0004"
created_at: 2026-04-01T03:13:40.147743+00:00
updated_at: 2026-04-02T12:35:42.837502+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: context-compaction-token-aware
---

# Context Compaction — token-aware conversation summarization Initiative

## Context

Long conversations hit the model's context window and fail. The engine currently sends the entire message history on every turn with no awareness of token limits. Without compaction, sessions are effectively limited to whatever fits in the context window — a hard ceiling that gets reached fast when tools produce large outputs (shell command results, file contents, grep results).

Claude Code solves this with automatic context compression when approaching token limits. Arawn needs the same.

### Reference
- Claude Code: CompactService compresses earlier messages while preserving key info
- ARAWN-I-0001: QueryEngine sends full session history on every turn
- ARAWN-I-0002: messages persisted as JSONL (compaction must integrate with persistence)

## Goals & Non-Goals

**Goals:**
- Token estimation for sessions (fast, approximate)
- Configurable context window limits per model
- Automatic compaction when estimated tokens exceed ~85% of context window
- Compaction via LLM summarization — older messages summarized, recent messages preserved verbatim
- Compaction fires as a separate step between engine turns, before `build_request`
- Compaction events persisted to JSONL so resumed sessions load the compacted version
- Works transparently — the engine and tools don't need to know compaction happened

**Non-Goals:**
- Exact token counting (approximate is fine — we're targeting a threshold, not a cliff)
- Streaming compaction (one-shot summarization call is fine)
- User-visible compaction controls (automatic only, no manual trigger in v1)
- Multiple compaction strategies (summarization only, no sliding window)

## Architecture

### Compaction Flow

```
Engine turn starts
  → Compactor checks: estimate_tokens(session) > threshold?
    → No: proceed normally
    → Yes:
      1. Split messages into "old" (to summarize) and "recent" (to keep)
      2. Send old messages to LLM with summarization prompt
      3. Replace old messages with a single Summary message
      4. Append a Compaction marker to JSONL
      5. Continue with compacted session
```

### Message Types

Add to `arawn_core::Message`:
```rust
Message::Summary {
    content: String,           // The LLM-generated summary
    original_count: usize,     // How many messages were summarized
    estimated_tokens_saved: u32, // Approximate tokens freed
}
```

The engine treats `Summary` like a `User` message when building requests — it's context the LLM should know about.

### Token Estimation

Simple heuristic: `tokens ≈ chars / 4`. Not exact but good enough for threshold decisions. Can be refined later with tiktoken or model-specific tokenizers.

### Compaction Prompt

Based on Claude Code's proven prompt (see `claude-code/src/services/compact/prompt.ts`):

**Structure:** The LLM produces an `<analysis>` scratchpad (stripped before entering context) followed by a `<summary>` with 9 structured sections:

1. Primary Request and Intent
2. Key Technical Concepts
3. Files and Code Sections (with snippets)
4. Errors and Fixes (with user feedback)
5. Problem Solving
6. All User Messages (non-tool-result)
7. Pending Tasks
8. Current Work (with verbatim quotes from recent conversation)
9. Optional Next Step

**Key prompt design choices:**
- Explicit "NO TOOLS" preamble — prevents LLM from calling tools during compaction
- `<analysis>` block improves summary quality but is stripped from the final output
- Post-compaction framing: "This session is being continued from a previous conversation..."
- Two modes: **full** (summarize everything) and **partial** (summarize old, keep recent verbatim)

### What to Keep vs Summarize

- **Keep verbatim:** last N messages (configurable, default 6 — roughly the last 2-3 turns)
- **Summarize:** everything before the keep window
- **Never summarize:** the system prompt (it's not in the message history)

### Persistence Integration

When compaction happens:
1. The JSONL file keeps all original messages (append-only, never rewritten)
2. A `Summary` message is appended after the compaction
3. On session load, the loader recognizes `Summary` messages and uses them as the starting point, skipping the messages they replaced

This means the full history is always recoverable from the JSONL, but the in-memory session uses the compacted version.

## Detailed Design

### TokenEstimator

```rust
struct TokenEstimator;

impl TokenEstimator {
    fn estimate_message(msg: &Message) -> u32 { /* chars / 4 */ }
    fn estimate_session(session: &Session) -> u32 { /* sum of messages */ }
    fn estimate_tools(tools: &[ToolDefinition]) -> u32 { /* schema chars / 4 */ }
}
```

### ModelLimits

```rust
struct ModelLimits {
    context_window: u32,        // e.g., 128_000 for llama-3.3
    compaction_threshold: f32,  // e.g., 0.85
}
```

Stored in `QueryEngineConfig`. Default threshold 85%.

### Compactor

```rust
struct Compactor {
    llm: Arc<dyn LlmClient>,
    keep_recent: usize,  // messages to preserve verbatim
}

impl Compactor {
    async fn should_compact(&self, session: &Session, limits: &ModelLimits, tool_tokens: u32) -> bool;
    async fn compact(&self, session: &mut Session, limits: &ModelLimits) -> Result<CompactionResult>;
}
```

### Integration with QueryEngine

The engine calls `compactor.should_compact()` at the top of each turn. If true, calls `compactor.compact()` which mutates the session in place (replaces old messages with Summary). The next `build_request` then sees the compacted history.

## Alternatives Considered

1. **Sliding window (drop old messages)** — Rejected. Lossy — the LLM loses context about earlier decisions. Summarization preserves the important bits.
2. **Token-exact counting (tiktoken)** — Rejected for v1. Adds a dependency and complexity. The 4-chars-per-token heuristic with an 85% threshold gives plenty of safety margin.
3. **Rewrite JSONL on compaction** — Rejected. Append-only is simpler, safer, and preserves full history for debugging. The Summary message acts as a checkpoint.
4. **Compact on every turn** — Rejected. Only compact when approaching the limit. Unnecessary summarization loses detail.

## Implementation Plan

Tasks to be decomposed after design approval:
1. Token estimator (message + session + tool definitions)
2. Message::Summary variant + JSONL serialization
3. Compactor (summarization logic, split old/recent, LLM call)
4. ModelLimits in QueryEngineConfig
5. Wire into engine loop (between-turn check)
6. Persistence integration (append Summary to JSONL, load with compaction awareness)
7. Tests (mock LLM compaction, threshold detection, session load with Summary)