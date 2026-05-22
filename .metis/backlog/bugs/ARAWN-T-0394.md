---
id: uat-scenario-schedule-with
level: task
title: "UAT scenario `schedule-with-confirmation` FAILs judge — agent hallucinates `gcal` instead of using `calendar_upcoming`"
short_code: "ARAWN-T-0394"
created_at: 2026-05-21T23:59:14.478883+00:00
updated_at: 2026-05-21T23:59:14.478883+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


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
- [x] P2 — quality issue, single scenario, doesn't block I-0053 close
- [ ] P3

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

*To be added during implementation*
