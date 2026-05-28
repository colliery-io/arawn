---
id: t-a-uat-integration-connection
level: task
title: "T-A: UAT integration-connection plumbing"
short_code: "ARAWN-T-0445"
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

# T-A: UAT integration-connection plumbing

## Parent Initiative
[[ARAWN-I-0062]]

## Objective
Make UAT capable of pretending a service is connected so the category-
visibility filter exposes its tools to the agent. No real client yet — just
the registration shim that flips `is_connected("calendar")` to `true`.

## Technical Approach
- Add `UatMockIntegration { service: String, connected: bool }` (or one type
  per service) under `crates/arawn-tests/tests/uat_mocks/` implementing the
  `Integration` trait with `is_connected() = true` and `name()` matching the
  canonical service slug (`google_calendar`, `gmail`, `slack`, …).
- Extend `Harness::start_server` (`uat.rs`) with
  `register_mock_integrations(services: &[&str])` — invoked before the engine
  boots — that inserts the mock into the existing `integration_registry`.
- Add the scenario-level field controlling which mocks are wired (e.g.
  `mock_integrations: Vec<String>` on `Scenario`, defaulting to empty).
- **Q1 decision**: introduce `MechanicalThresholds.required_tool_names:
  Vec<String>` so a scenario can assert "the agent must have called this tool
  by name." Mirrors the existing `required_evidence` (content-grep) substrate.
- Add a scaffold test scenario `mock-integrations-visible` that registers
  Calendar, sends "list your tools," and asserts the assistant transcript
  mentions a `calendar_*` tool name.

## Acceptance Criteria
- [ ] `connected_services` reports the mocked services as connected when
      `mock_integrations` is non-empty.
- [ ] Integration tools for those services land in the live registry and pass
      `filter_tools_for_context` visibility for that scenario.
- [ ] `required_tool_names` mechanical check implemented; scenarios that don't
      use it default to empty.
- [ ] Scaffold scenario green: integration tool category visible to the agent.
- [ ] `angreal check workspace` clean.

## Dependencies
Blocks every other task in this initiative. T-B/C/D/E/F/G all need the
registration shim + `required_tool_names`.

## Status Updates
*To be added during implementation*
