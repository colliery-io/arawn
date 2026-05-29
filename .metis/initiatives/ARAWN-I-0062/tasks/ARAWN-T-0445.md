---
id: t-a-uat-integration-connection
level: task
title: "T-A: UAT integration-connection plumbing"
short_code: "ARAWN-T-0445"
created_at: 2026-05-28T22:01:44.832147+00:00
updated_at: 2026-05-29T00:43:06.013983+00:00
parent: ARAWN-I-0062
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

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

**2026-05-28 — Done (plumbing only).** Landed:

- `arawn_integrations::UatMockIntegration` — lifecycle-only `Integration` impl
  reporting `is_connected() = true` and a `capabilities_summary` tagged
  `uat-mock`. Lives at `crates/arawn-integrations/src/uat_mock.rs`, exported
  from `lib.rs`. Unit-tested.
- Server-side wiring: `startup::integrations::wire_uat_mock_integrations`
  reads `ARAWN_UAT_MOCK_INTEGRATIONS` (comma-separated service slugs) and
  registers a mock per name. Called from `main.rs` after the OAuth
  `wire_integrations`.
- Harness-side: `Scenario.mock_integrations: Vec<String>` (serde-default);
  `Harness::start_server_with(&[...])` injects the env var when non-empty;
  the test runner passes `scenario.mock_integrations` through.
- New `MechanicalThresholds.required_tool_names: Vec<String>` field +
  `MechanicalCheckResult.missing_tool_names`. Mechanical check tracks the
  union of every `tool_call.name` across all turns and fails if any required
  name was never called. Surfaced in the summary print line ("Missing tool
  calls: …") alongside "Missing evidence: …".
- All 15 existing scenarios get `mock_integrations: vec![]` and
  `required_tool_names: vec![]` (no behavioural change).

**Architectural finding for T-B+:** the "tools land in registry" half of T-A's
acceptance is NOT met, and that's correct — it's not achievable yet.
Integration TOOLS are registered in
`crates/arawn/src/startup/integrations.rs` from typed concrete integrations
(`GmailIntegration::new(client_id, client_secret)` etc.), guarded by
*credentials present*, and they take `Arc<GmailIntegration>` (not
`Arc<dyn Integration>`) at construction. A bare lifecycle-mock won't bring
those tools into the registry — T-B/T-C/T-D each need to build a typed
concrete-integration variant with a fake provider client and wire it into the
same registration path the OAuth integrations use today, so the tools get
constructed and registered against the mock client. That's exactly what T-B
(calendar) is sized for; T-A's contribution is the shared substrate (env-var
+ harness + mechanical check) those tasks build on.

Verification: `arawn-integrations::uat_mock::tests::reports_connected_and_named`
green; `cargo build --tests` clean; `angreal check workspace` exit 0.