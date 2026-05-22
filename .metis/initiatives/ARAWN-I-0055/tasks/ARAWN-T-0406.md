---
id: t-b-capability-driven-filter
level: task
title: "T-B: Capability-driven filter branch for integration categories"
short_code: "ARAWN-T-0406"
created_at: 2026-05-22T16:36:01.000000+00:00
updated_at: 2026-05-22T16:36:01.000000+00:00
parent: ARAWN-I-0055
blocked_by: [ARAWN-T-0405]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0055
---

# T-B: Capability-driven filter branch for integration categories

## Parent Initiative

[[ARAWN-I-0055]]

## Objective

Rewrite the integration-category branch of `filter_tools_for_context` so integration tools are included iff the corresponding capability is in the connected set. No keyword scan for integration categories.

## Acceptance Criteria

- [ ] `crates/arawn-engine/src/query_engine.rs::filter_tools_for_context` includes integration tools using capability lookup, not keyword scan.
- [ ] The capability provider (`config.prompt_context.integration_capabilities`) is queried once per filter call (already once per turn; this is the same frequency).
- [ ] Mapping: `ToolCategory::Calendar` → look for `"calendar"` in connected capabilities; same per-service mapping for `Gmail`, `Drive`, `Slack`, `Atlassian`, `GitHub`. Exact capability names verified against the existing provider impl in `crates/arawn/src/local_service/mod.rs:416`.
- [ ] Unit tests added to `query_engine.rs` `mod tests`:
  - [ ] `calendar_tool_visible_when_calendar_capability_present_no_keywords`: build session with messages.len() > 2, user message contains "Bob is free Tue mornings" (no calendar/web keywords), capability set = `["calendar"]`. Assert `calendar_upcoming` is in the filtered list.
  - [ ] `calendar_tool_hidden_when_calendar_capability_absent`: same setup, capability set = `[]`. Assert `calendar_upcoming` is NOT in the filtered list.
  - [ ] Equivalent positive+negative pair for one other integration (e.g., Slack) to confirm the pattern.
- [ ] `cargo check --workspace` clean.
- [ ] `cargo test --workspace --lib` green — new tests pass alongside existing 1,758.

## Implementation Notes

- Depends on T-A landing first (the per-service categories must exist).
- The capability provider returns an `IntegrationCapabilities` value. Confirm the type's API for "is service connected" — likely either a `HashSet<String>` or `Vec<IntegrationCapability>` with a `name()` accessor. Use whatever is present.
- Edge case: if `integration_capabilities` is `None` (no provider set — happens for the `startup/engine.rs` default config used by single-shot CLI runs), default to **excluding** integration tools. This is consistent with "no provider == we don't know what's connected, be conservative."

## Status Updates

*To be added during implementation*
