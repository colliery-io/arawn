---
id: t-c-audit-expand-non-integration
level: task
title: "T-C: Audit + expand non-integration keyword sets"
short_code: "ARAWN-T-0407"
created_at: 2026-05-22T16:36:02+00:00
updated_at: 2026-05-22T18:12:46.003985+00:00
parent: ARAWN-I-0055
blocked_by: [ARAWN-T-0406]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0055
---

# T-C: Audit + expand non-integration keyword sets

## Parent Initiative

[[ARAWN-I-0055]]

## Objective

Walk each non-integration `ToolCategory`'s keyword list in `filter_tools_for_context` and expand to cover legitimate prompts that the current set misses. Add a negative-test per category asserting expected routing behavior.

## Acceptance Criteria

## Acceptance Criteria

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

### 2026-05-22 — landed

**Keyword expansions in `filter_tools_for_context`:**

| Category | Original | T-C additions |
|---|---|---|
| Plan | `plan` | `design`, `approach`, `strategy` |
| Task | `task`, `todo`, `background` | `queue` |
| Agent | `agent`, `delegat` | `subagent`, `spawn` |
| Ceremony | `retro`, `ceremony`, `standup`, `diary`, `daily`, `today`, `brief`, `weekly`, `week`, `priorities`, `priority`, `todo`, `reminder`, `remind me` | `agenda`, `morning`, `afternoon`, `tomorrow`, `yesterday` |
| Web | `http`, `url`, `web`, `search`, `fetch`, `api` (already narrowed in T-B — `github`, `google` dropped because those were proxies for integration tools now capability-gated) | — |
| Memory | `remember`, `recall`, `memory`, `forget` | — (will go always-on in T-D) |
| Workstream | `workstream`, `workspace` | — (will go always-on in T-D) |

Each category now has an inline comment naming the T-C additions and the reasoning ("read as X without the literal word Y").

**Unit tests (19 new):**
- Two helpers — `assert_tool_visible(cat, name, user_msg)` and `assert_tool_hidden(...)` — that build a registry with one stub tool, run the filter against a session past iter-1, and assert membership. Keeps the tests DRY.
- Web: positive (`fetch the URL`), negative (`say hi to Bob`), explicit regression (`open the github repo` → hidden, since `github` was dropped as a Web trigger).
- Plan: positive on `plan`, positive on T-C `design`, negative.
- Task: positive on T-C `queue`, negative.
- Memory: positive on `recall`, negative.
- Agent: positive on T-C `subagent`, positive on T-C `spawn`, negative.
- Workstream: positive on `workstream`, negative.
- Ceremony: positive on T-C `agenda`, positive on T-C `tomorrow`, positive on T-C `morning`, negative.

**Validation:**
- `cargo check --workspace`: ✅ clean.
- `cargo build --workspace --release`: ✅ clean (1m 13s).
- `cargo test -p arawn-engine --lib query_engine`: ✅ **32 tests pass** (was 9 pre-T-B; 13 after T-B; 32 after T-C).
- `cargo test --workspace --lib`: ✅ **1,781 tests pass**, 0 fail (1,758 baseline + 4 T-B + 19 T-C).

Side-effect of the test isolation: noticed one harness test (`testing::harness::tests::harness_shell_tool_receives_arguments`) is order-sensitive — it failed on one workspace run but passed on a re-run and in isolation. Not introduced by T-C; pre-existing flake worth noting.