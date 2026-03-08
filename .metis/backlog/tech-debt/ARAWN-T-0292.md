---
id: add-end-to-end-agent-tests
level: task
title: "Add end-to-end agent tests"
short_code: "ARAWN-T-0292"
created_at: 2026-03-08T20:21:14.234061+00:00
updated_at: 2026-03-08T20:21:14.234061+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Add end-to-end agent tests

## Objective

Agent tests currently mock everything — no test verifies the full flow of tool dispatch → execution → response assembly. Add end-to-end tests that exercise the agent loop with real (but lightweight) tool implementations.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P1 - High (important for user experience)

### Technical Debt Impact
- **Current Problems**: 87 agent tests exist but all use mocks. Integration bugs between tool dispatch and response assembly are invisible.
- **Benefits of Fixing**: Catches integration bugs in the agent loop, validates tool results flow back correctly.
- **Risk Assessment**: Medium — mocked tests cover unit logic but miss wiring issues.

## Acceptance Criteria

- [ ] At least one test exercises: user message → tool selection → tool execution → response with tool output
- [ ] Test covers multi-turn conversation (tool call followed by follow-up)
- [ ] Test covers tool error handling (tool returns error, agent handles gracefully)
- [ ] Tests use lightweight mock LLM backend (not real API calls)
- [ ] `cargo test -p arawn-agent` passes
- [ ] `angreal check clippy` clean

## Implementation Notes

### Technical Approach
- Create a test-only LLM backend that returns deterministic tool-call responses
- Wire it into the real Agent with real tool registry
- Register simple test tools (echo, fail, etc.)
- Verify the full round-trip through the agent loop

### Files
- `crates/arawn-agent/src/agent.rs` or `crates/arawn-agent/tests/`

## Status Updates

*To be added during implementation*