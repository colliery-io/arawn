---
id: t-a-add-per-service-toolcategory
level: task
title: "T-A: Add per-service ToolCategory variants + re-categorize integration tools"
short_code: "ARAWN-T-0405"
created_at: 2026-05-22T16:36:00.000000+00:00
updated_at: 2026-05-22T16:36:00.000000+00:00
parent: ARAWN-I-0055
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0055
---

# T-A: Add per-service ToolCategory variants + re-categorize integration tools

## Parent Initiative

[[ARAWN-I-0055]]

## Objective

Add `Calendar`, `Gmail`, `Drive`, `Slack`, `Atlassian`, `GitHub` variants to `arawn_tool::ToolCategory` and move every integration tool impl off the `Web` category onto its per-service variant. Leaves `Web` for actual web tools (`web_fetch`, `web_search`).

## Acceptance Criteria

- [ ] `ToolCategory` enum in `crates/arawn-tool/src/` has 6 new variants: `Calendar`, `Gmail`, `Drive`, `Slack`, `Atlassian`, `GitHub`.
- [ ] Every `impl Tool for X` under `crates/arawn-integrations/src/{calendar,gmail,drive,slack,atlassian,github}/tools.rs` returns its per-service category from `fn category()`.
- [ ] After the sweep, the only `ToolCategory::Web` impls are `web_fetch` and `web_search` (in `crates/arawn-engine/src/tools/`).
- [ ] `cargo check --workspace` clean.
- [ ] `cargo build --workspace --release` clean.
- [ ] `cargo test --workspace --lib` green — all existing tests still pass (no behavior change; filter logic in T-B will catch up).

## Implementation Notes

- This task is mechanical sweep + enum extension. No filter logic changes yet — `filter_tools_for_context` will continue to handle the new categories via the default-keyword branch until T-B lands. That means temporarily, integration tools will be **always dropped** in iter-2+ (because no keyword set matches their new categories). UAT will fail on schedule-with-confirmation more reliably during this window — that's expected. T-B is the fix.
- Audit step: before promoting, `grep -rn "ToolCategory::Web" crates/arawn-integrations/` to inventory every site. Verify count matches the per-service distribution after the change.
- Keep `ToolCategory::Web` itself in the enum — `web_fetch`/`web_search` still need it.

## Status Updates

*To be added during implementation*
