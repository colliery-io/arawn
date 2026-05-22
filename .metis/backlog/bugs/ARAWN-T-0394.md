---
id: uat-scenario-schedule-with
level: task
title: "UAT scenario `schedule-with-confirmation` FAILs judge — agent hallucinates `gcal` instead of using `calendar_upcoming`"
short_code: "ARAWN-T-0394"
created_at: 2026-05-21T23:59:14.478883+00:00
updated_at: 2026-05-22T18:28:27.506346+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# UAT scenario `schedule-with-confirmation` FAILs judge — agent hallucinates `gcal` instead of using `calendar_upcoming`

## Backlog Item Details

### Type
- [x] Bug — Production issue that needs fixing (UAT regression)
- [ ] Feature
- [ ] Tech Debt
- [ ] Chore

### Priority
- [ ] P0
- [ ] P1
- [ ] P2
- [x] P3 — downgraded 2026-05-22 after follow-up UAT pass. The scenario passed cleanly at I-0054 close with the same `gemma4:31b-cloud` and same I-0053-fixed code (I-0054 was pure restructuring). The failure is non-deterministic — model flakiness, not a code regression. Track and address only if the flake rate becomes high enough to matter.

### Impact Assessment

- **Affected scenarios**: 1 of 13 UAT scenarios (`schedule-with-confirmation`).
- **Reproduction**: `angreal test uat` then `angreal test uat-judge --results /tmp/arawn-uat-...`. The judge marks this scenario `pass: false` with `completion=2/5`, `quality=1/5`.
- **Failure mode**: agent never proposes a 30-min meeting slot. Behavioral trace:
  1. Tries `gcal` (shell command — doesn't exist, hallucinated).
  2. Tries `signal_search` (returns empty — correct behavior, no signal data).
  3. Tries `weekly_run` for current week (wrong tool for calendar lookup; also wrong week).
  4. **Never calls `calendar_upcoming`** — the actually-registered tool that would have answered.
  5. Produces no user-facing assistant_text.
- **Mechanical PASSed** (max_tool_errors=3, the agent stayed under that). Judge FAILed because the qualitative outcome — "propose a specific slot" — was never delivered.

### Identified during I-0053
First surfaced during the UAT run at the close of ARAWN-I-0053 (post-iteration cruft removal). The 12 cleanup tasks did NOT touch calendar tools, tool registration, the system prompt's tool-listing path, or the assistant identity layer. The `PromptSection.name` field I removed in T-C was never read (only set), so cleanup is not the cause. This appears to be a pre-existing model-quality issue with `gemma4:31b-cloud` that was just never surfaced before — the scenario was added in `1c5e6ec` (I-0035 personal-assistant identity work) and has only been UAT-judged a small number of times since.

## Objective

Diagnose and fix the failure mode so the `schedule-with-confirmation` scenario passes judge with `gemma4:31b-cloud` (or document why it can't, and migrate to a model that can).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Read the full transcript at `/tmp/arawn-uat-20260521-175626/schedule-with-confirmation/uat-results/schedule-with-confirmation/gemma4:31b-cloud/transcript.jsonl`. Confirm whether `calendar_upcoming` was in the tool list presented to the model on turn 1.
- [ ] If `calendar_upcoming` WAS in the tool list: the issue is model-prompt level. Options:
  - Sharpen the system prompt to nudge the model toward calendar tools when the user mentions a calendar action.
  - Add an example use-case in the calendar tool's `description()` so the small model can pattern-match.
  - Accept that small models hallucinate and either swap models or mark this scenario as known-bad on small models.
- [ ] If `calendar_upcoming` WAS NOT in the tool list: there's a tool-filtering bug — the engine's keyword filter dropped it because the user's prompt didn't contain "calendar" verbatim. Fix the filter or expand its trigger words.
- [ ] Re-run `angreal test uat --scenario schedule-with-confirmation` and `angreal test uat-judge` against just that scenario. Verify `pass: true`.

## Implementation Notes

### Hypothesis ranking

1. **Most likely (tool-filter bug)**: the engine's tool-list filter in `query_engine.rs` or `system_prompt.rs` only surfaces tools whose `category()` or `name()` matches keywords in the user message. The user's prompt mentions "calendar" once but the surrounding context is "catch-up email" / "30-min slot" — the filter might be dropping calendar tools. **First step:** read the transcript to confirm.
2. **Less likely (model issue)**: small Gemma models hallucinate tool names when the actual tool is in scope but unnamed in the user prompt. Mitigation would be a richer system prompt or a model swap.
3. **Least likely**: a regression from I-0053 cleanup. Already ruled out architecturally — my cleanup commits did not touch any of: calendar tools, `register_default_tools` in main.rs, system prompt section assembly, model-hint resolution, or capability-based tool filtering.

### Where to start

- `crates/arawn/src/main.rs::register_default_tools` — confirm `calendar_*` tools are registered when integrations.calendar credentials are present.
- `crates/arawn-engine/src/query_engine.rs` — find the tool-listing filter; check what triggers calendar visibility.
- `crates/arawn-engine/src/system_prompt.rs::tools()` — how `ToolDefinition::name` appears to the model.

### Dependencies

None. Standalone backlog bug.

### Risk Considerations

- If this is a tool-filter bug, the fix may surface OTHER scenarios that were passing only because they got lucky with the filter. Worth re-running the full UAT after any filter change.

## Status Updates

### 2026-05-22 — UAT re-run: PASS (flaky, not regressed)

At I-0054 close (module restructuring — pure code reorganization, no behavioral changes), the same scenario ran with the same `gemma4:31b-cloud` and **passed clean**:
- `completion=4/5`, `quality=4/5`, `pass=true`
- Judge summary: "Switched to personal, searched feeds/calendar, proposed specific 30-min slot (Tue May 26 9-9:30 AM) and asked for confirmation before booking."
- Judge artifact: `/tmp/arawn-uat-20260522-124911/schedule-with-confirmation/uat-results/schedule-with-confirmation/gemma4:31b-cloud/judge.json`

This confirms hypothesis #2 from the bug doc: the failure mode is **non-deterministic model behavior**, not a tool-filtering bug. The small model sometimes hallucinates `gcal` and sometimes picks `calendar_upcoming` correctly — there's no fix at the engine layer until we either (a) richen the calendar tool descriptions to make pattern-matching more reliable, (b) sharpen the system prompt, or (c) swap to a larger model that doesn't hallucinate.

Downgraded P2 → P3. Track flake rate across future UAT runs; promote back to P2 if the scenario fails ≥2 of next 5 runs.

### 2026-05-22 — closed via I-0055 T-F regression test

**Root cause reassessment:** the flake was NOT just non-deterministic model behavior. Per investigation under ARAWN-I-0055:

1. On iter-1 of turn 1, the agent saw all tools (filter early-return at `session.messages().len() <= 2`). Sometimes it picked `calendar_upcoming` correctly; sometimes it picked `shell("gcal")` — that's the flaky model-decision part.
2. From iter-2 onward, the filter activated. Calendar tools were tagged `ToolCategory::Web` and the Web keyword set didn't match a calendar prompt, so calendar tools got **dropped from the catalog**.
3. The agent had no recovery path. Once iter-1 misfired, every subsequent iteration scanned a catalog that no longer contained `calendar_upcoming`, so it flailed into `weekly_run` or `shell` and never recovered.

**Fix landed in I-0055 T-A + T-B:**
- T-A re-categorized calendar tools off `Web` onto a new `ToolCategory::Calendar` (plus 5 sibling per-service categories).
- T-B made integration tools capability-gated: when `google_calendar` is in the connected capabilities set, calendar tools survive iter-2+ unconditionally regardless of user message text.

**Regression test:** `query_engine::tests::t_0394_calendar_tools_survive_filter_after_iter_1_when_calendar_capability_connected` in `crates/arawn-engine/src/query_engine.rs`. Uses the literal failing user prompt from the original transcript. Companion negative test `t_0394_calendar_tools_hidden_when_capability_absent` confirms the capability gate works both ways.

Closing this backlog bug. The flake mode is structurally blocked.