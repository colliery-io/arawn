---
id: integration-tests-memory-stack
level: task
title: "Integration tests — memory stack injection, budget limits, shortcode compression"
short_code: "ARAWN-T-0132"
created_at: 2026-04-09T16:28:59.230993+00:00
updated_at: 2026-04-09T16:42:40.022554+00:00
parent: ARAWN-I-0019
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0019
---

# Integration tests — memory stack injection, budget limits, shortcode compression

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0019]]

## Objective

End-to-end integration tests verifying the full memory stack pipeline: L0/L1 generation, budget enforcement, shortcode compression, L2 injection, and deduplication across layers.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Test in `crates/arawn-tests/tests/memory_stack.rs`
- [ ] Test: populate KB with entities of varying confidence, call `wake_up(900)`, verify output is under budget and highest-confidence entities appear first
- [ ] Test: populate KB with repeated entity names, verify shortcodes are applied in L1 output
- [ ] Test: send a message mentioning a KB entity tag, verify L2 context is injected into the system prompt
- [ ] Test: L2 entities already in L1 are not duplicated
- [ ] Test: empty KB produces minimal L0 output (workstream metadata only), no L1/L2
- [ ] Test: wake_up with very small budget (50 tokens) still produces valid output without panicking

### Key files
- `crates/arawn-tests/tests/memory_stack.rs` — new test file

### Dependencies
- T-0129, T-0130, T-0131 (all layers must be implemented)

## Status Updates

*To be added during implementation*