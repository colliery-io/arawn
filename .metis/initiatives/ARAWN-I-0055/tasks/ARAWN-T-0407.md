---
id: t-c-audit-expand-keyword-sets
level: task
title: "T-C: Audit + expand non-integration keyword sets"
short_code: "ARAWN-T-0407"
created_at: 2026-05-22T16:36:02.000000+00:00
updated_at: 2026-05-22T16:36:02.000000+00:00
parent: ARAWN-I-0055
blocked_by: [ARAWN-T-0406]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0055
---

# T-C: Audit + expand non-integration keyword sets

## Parent Initiative

[[ARAWN-I-0055]]

## Objective

Walk each non-integration `ToolCategory`'s keyword list in `filter_tools_for_context` and expand to cover legitimate prompts that the current set misses. Add a negative-test per category asserting expected routing behavior.

## Acceptance Criteria

- [ ] Each non-integration category (Ceremony, Web, Plan, Task, Memory, Agent, Workstream — note Memory/Workstream may be promoted to always-on in T-D, in which case skip them here) has an audited keyword set documented inline (one comment per category citing what was added and why).
- [ ] Specific minimum additions:
  - [ ] `Ceremony`: add "agenda", "morning", "afternoon", "tomorrow", "yesterday", "this week", "next week".
  - [ ] `Web`: narrow to `http, url, web, search, fetch, api` (drop `github, google` — those were proxies for integration tools that are now capability-gated in T-B).
  - [ ] `Plan`: add "design", "approach", "strategy".
  - [ ] `Task`: add "queue".
  - [ ] `Agent`: add "subagent", "spawn".
- [ ] Unit tests in `query_engine.rs` `mod tests`:
  - [ ] Per category: positive test (user message contains a keyword → category active → relevant tool visible).
  - [ ] Per category: negative test (user message contains NO keywords → category inactive → relevant tool dropped).
- [ ] `cargo test --workspace --lib` green.

## Implementation Notes

- This task can be sequenced after T-B (capability-driven integration) but is independent of T-D/T-E.
- The keyword sets live in `filter_tools_for_context` itself — easy to edit, easy to test.
- Note the `week` keyword in Ceremony also matches "next week" naturally — no need for the multi-word match. Keep both for clarity.

## Status Updates

*To be added during implementation*
