---
id: t-g-sharpen-required-evidence
level: task
title: "T-G: Sharpen required_evidence across converted scenarios"
short_code: "ARAWN-T-0451"
created_at: 2026-05-28T22:01:44.832147+00:00
updated_at: 2026-05-28T22:01:44.832147+00:00
parent: ARAWN-I-0062
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0062
---

# T-G: Sharpen required_evidence across converted scenarios

## Parent Initiative
[[ARAWN-I-0062]]

## Objective
Once the live-tool path is exercised, walk every converted scenario and
ratchet up `required_evidence` + `required_tool_names` so the mechanical
check tests the **substantive** answer, not just non-empty output. Each
scenario should fail mechanically if the production-shaped answer isn't
produced.

## Scope
- Inventory each converted scenario (`morning-briefing`, `inbox-summary`,
  `mention-scan`, `draft-with-confirmation`, `schedule-with-confirmation`,
  plus anything that picks up the Drive stub).
- For each, identify the substrings / tool names that prove the **live path
  was taken** AND the substantive payload was retrieved.
- Add or extend the corresponding mechanical fields.
- Document any scenario that genuinely cannot get sharper without bloating
  the judge rubric.

## Acceptance Criteria
- [ ] Every converted scenario has `required_tool_names` populated for the
      live-tool calls it expects.
- [ ] Every converted scenario has `required_evidence` substrings that prove
      the payload was retrieved through the live tool (not the corpus
      fallback).
- [ ] A `missing_evidence` or missing-tool result clearly explains *which*
      live tool/payload the run missed.
- [ ] `angreal check workspace` + `angreal test uat` green on the converted
      scenarios.

## Dependencies
Closes the initiative — depends on T-A through T-F.

## Status Updates
*To be added during implementation*
