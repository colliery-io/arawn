---
id: per-message-memory-refresh-and-l2
level: task
title: "Per-message memory refresh and L2 auto-injection from user message keywords"
short_code: "ARAWN-T-0131"
created_at: 2026-04-09T16:28:58.229761+00:00
updated_at: 2026-04-09T16:40:22.824435+00:00
parent: ARAWN-I-0019
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0019
---

# Per-message memory refresh and L2 auto-injection from user message keywords

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0019]]

## Objective

Add L2 topic-triggered context loading. When the user's message mentions entities or tags that match KB entries not already in L1, auto-inject a `[L2 — CONTEXT]` section before the LLM call. Also add `retrieve_topical()` to MemoryManager for filtered retrieval with a token budget.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `MemoryManager::retrieve_topical(keywords: &[String], budget_tokens: usize) -> Vec<Entity>` searches both tiers by tag/title match, returns within budget
- [ ] `local_service.rs:send_message()` extracts keywords from user message text, calls `retrieve_topical()`, injects as L2 section in PromptContext
- [ ] L2 entities that are already in L1 are deduplicated (not injected twice)
- [ ] L2 budget capped at ~400 tokens
- [ ] L2 section formatted as `[L2 — CONTEXT]\n` with entity summaries
- [ ] When no topical matches found, no L2 section injected (no empty block)
- [ ] Unit test: keyword "arawn-engine" retrieves entities tagged with "arawn-engine"
- [ ] Unit test: L2 deduplicates against L1 entities

### Key files
- `crates/arawn-memory/src/manager.rs` — add `retrieve_topical()`
- `crates/arawn-memory/src/stack.rs` — add `topical_context()` method
- `crates/arawn/src/local_service.rs` — inject L2 in send_message before engine.run()

### Dependencies
- T-0129 (MemoryStack and L1 must exist for deduplication)

## Status Updates

*To be added during implementation*