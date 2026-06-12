---
id: p1-8-9-10-engine-small-batch
level: task
title: "P1-8/9/10: Engine small-batch — sticky tool set, ask-without-prompter diagnosability, duplicate-call detection"
short_code: "ARAWN-T-0474"
created_at: 2026-06-11T11:07:24.205039+00:00
updated_at: 2026-06-11T18:47:42.398271+00:00
parent: ARAWN-I-0067
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0067
---

# P1-8/9/10: Engine small-batch — sticky tool set, ask-without-prompter diagnosability, duplicate-call detection

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0067]] — implements findings P1-8, P1-9, P1-10 (all MEDIUM). Three small, independent engine fixes batched as one task.

## Objective **[REQUIRED]**

Close three engine-loop friction points: (a) tools used earlier in a session can vanish from later turns' tool lists; (b) Ask-mode permission checks with no prompter attached look like ordinary denials; (c) duplicate-failing-call detection is fragile.

**Current defects:**
- `crates/arawn-engine/src/query_engine.rs:1115-1200` — after turn 1, tools are filtered by keyword/category match against the new user message. A tool used successfully last turn can be dropped this turn, producing "tool not registered" rejections for a tool that was available moments ago (P1-8).
- `crates/arawn-engine/src/permissions/checker.rs:448-488` — when a rule resolves to Ask and no prompter is configured, the checker logs a warning and returns Denied; nothing tells the agent or user that the real cause is a missing prompter wiring (P1-9).
- `crates/arawn-engine/src/query_engine.rs:485-500` — duplicate detection keys on exact argument hashes (whitespace variation resets the count) and `failed_call_counts` accumulates without session-scoped clearing if the engine is reused (P1-10).

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] Tools used earlier in a session stay available on later turns (`filter_tools_for_context` force-includes any name in earlier Assistant `tool_uses` — **already implemented**, lines ~1356-1368). Regression test `previously_used_tools_stay_available_under_filtering` added.
- [x] Ask-with-no-prompter is now diagnosable: new `DecisionReason::AskWithoutPrompter` threaded through both Ask paths in `check_explained`, with display "permission 'ask' was required but no interactive prompter is attached (a wiring/configuration issue — not a user denial)". Test `check_explained_ask_without_prompter_is_diagnosable`.
- [x] Duplicate-call detection is already canonical: the key re-serializes args via `serde_json::Value` Display (sorted keys — `preserve_order` off — and whitespace normalized), and `failed_call_counts` lives on the engine which is **rebuilt per message**, so counts are naturally per-turn-loop scoped. Clarifying comment added + test `duplicate_call_key_is_canonical_and_order_independent`.
- [x] Inline regression tests for all three (2 in query_engine, 1 in checker).
- [x] `angreal test unit` green for changed crate (arawn-engine **733/0**, incl. the sandbox tests this run; arawn 67/0+2/0); `angreal check all` carries pre-existing repo-wide failures.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Sticky tools: maintain a per-session `HashSet` of tool names that have been called; union it into the post-filter tool list in `query_engine.rs:1115-1200`. Ask-without-prompter: return a distinct permission outcome/message from `checker.rs` so the denial text names the missing prompter. Duplicate detection: canonicalize arguments by parsing to `serde_json::Value` and re-serializing before hashing; key counts by session and clear on session end.

### Dependencies
Touches `query_engine.rs` — land **after** ARAWN-T-0468 and ARAWN-T-0473 to avoid conflicts (smallest task, most flexible).

### Risk Considerations
The sticky set grows the tool list and therefore token usage in long sessions — acceptable at Phase 1 scale; note the interaction for ARAWN-T-0274's compaction layer. Keep the three fixes as separate commits so any one can be reverted independently.

## Status Updates **[REQUIRED]**

**2026-06-11 — COMPLETE.** One real fix (P1-9) plus verification + regression tests for two findings that turned out already-correct in the current code.

- **P1-9 (ask-without-prompter) — fixed.** Added `DecisionReason::AskWithoutPrompter` and threaded it through both `Ask` resolution paths in `check_explained` (the direct-rule Ask and the NoMatch→mode-fallback Ask). When a check resolves to Ask but `self.prompter.is_none()`, the denial now carries this distinct reason whose `display()` explains it's a wiring/config problem, not a user denial — so the agent/operator can tell the two apart in the audit log and denied-hook message.
- **P1-8 (sticky tools) — already correct.** `filter_tools_for_context` already collects `used_tool_names` from earlier Assistant `tool_uses` and force-includes both those exact tool names and their categories (lines ~1356-1368). The reviewer flagged this reading only the filter's keyword head and missing the sticky tail. Added a regression test to fence it.
- **P1-10 (duplicate-call key) — already robust.** The key `format!("{}:{}", name, args)` re-serializes `serde_json::Value`, whose Display is canonical: `preserve_order` is OFF (verified — Value is a sorted BTreeMap) and raw-output whitespace is normalized at parse time, so `{"a":1,"b":2}` and `{ "b":2, "a":1 }` collapse to one key. And `failed_call_counts` is on the `QueryEngine`, which `LocalService::build_engine` constructs fresh per message — so counts are already scoped to a single turn-loop, never leaking across sessions. Added a clarifying comment + a canonicalization test.

**Tests:** `check_explained_ask_without_prompter_is_diagnosable`, `previously_used_tools_stay_available_under_filtering`, `duplicate_call_key_is_canonical_and_order_independent`.

**Verification:** `cargo build` clean; arawn-engine **733 passed / 0 failed** (the two sandbox-exec flaky tests both passed this run), arawn 67/0 + 2/0; checker.rs and query_engine.rs `rustfmt --check` clean; no new dependency.