---
id: ranked-entity-query-and
level: task
title: "Ranked entity query and MemoryStack with L0/L1 token-budgeted generation"
short_code: "ARAWN-T-0129"
created_at: 2026-04-09T16:28:55.361969+00:00
updated_at: 2026-04-09T16:37:27.145274+00:00
parent: ARAWN-I-0019
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0019
---

# Ranked entity query and MemoryStack with L0/L1 token-budgeted generation

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0019]]

## Objective

Add a `list_all_ranked()` query to MemoryStore and build `MemoryStack` — a new struct in arawn-memory that generates token-budgeted L0 (identity) and L1 (essential facts) output from the KB. Wire it into `local_service.rs` to replace the current flat injection.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `MemoryStore::list_all_ranked(limit)` queries non-superseded entities ordered by confidence source (stated > observed > inferred), reinforcement count, then recency
- [ ] New `crates/arawn-memory/src/stack.rs` with `MemoryStack` struct
- [ ] `MemoryStack::wake_up(budget_tokens: usize) -> String` generates L0 + L1 output within budget
- [ ] L0 renders workstream metadata + Person/Convention entities (~100 tokens)
- [ ] L1 renders top-ranked entities grouped by type, filling remaining budget
- [ ] Token estimation uses `text.len() / 4` (matching existing `TokenEstimator`)
- [ ] `local_service.rs:send_message()` calls `MemoryStack::wake_up()` per-message instead of reusing stale startup memories
- [ ] `main.rs` startup injection replaced with stack-based approach
- [ ] Unit test: wake_up respects token budget, doesn't exceed it
- [ ] Unit test: L1 ranking puts stated-confidence entities before inferred

### Key files
- `crates/arawn-memory/src/store.rs` — add `list_all_ranked()`
- `crates/arawn-memory/src/stack.rs` — new file
- `crates/arawn-memory/src/lib.rs` — export MemoryStack
- `crates/arawn/src/local_service.rs` — per-message injection
- `crates/arawn/src/main.rs` — remove stale startup injection

## Status Updates

*To be added during implementation*