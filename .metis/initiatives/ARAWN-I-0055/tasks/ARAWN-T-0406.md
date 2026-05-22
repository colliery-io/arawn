---
id: t-b-capability-driven-filter
level: task
title: "T-B: Capability-driven filter branch for integration categories"
short_code: "ARAWN-T-0406"
created_at: 2026-05-22T16:36:01+00:00
updated_at: 2026-05-22T18:00:15.150731+00:00
parent: ARAWN-I-0055
blocked_by: [ARAWN-T-0405]
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: ARAWN-I-0055
---

# T-B: Capability-driven filter branch for integration categories

## Parent Initiative

[[ARAWN-I-0055]]

## Objective

Rewrite the integration-category branch of `filter_tools_for_context` so integration tools are included iff the corresponding capability is in the connected set. No keyword scan for integration categories.

## Acceptance Criteria

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

### 2026-05-22 — landed

**Approach refinement (vs original plan):** the original plan said "use the existing `integration_capabilities` provider". But that provider returns prose summaries (`"slack (connected; bot scopes: …)"`), and Calendar/Gmail don't override `capabilities_summary` at all — they return `None`, meaning a filter that depended on summaries would never see Calendar/Gmail capabilities. Better split: keep `integration_capabilities` for the system prompt's verbose summaries, add a parallel `connected_services` provider that returns clean `Integration::name()` tokens for the filter.

**New plumbing:**
- `crates/arawn-engine/src/query_engine.rs`: added `ConnectedServicesFn` type alias; added `pub connected_services: Option<ConnectedServicesFn>` to `PromptContext`. Exported from `lib.rs`.
- `crates/arawn/src/local_service/mod.rs`: new closure populates `connected_services` by walking the integration registry and calling `is_connected().await` on each. Same `block_in_place` pattern as the existing capabilities-summary closure.
- `crates/arawn/src/startup/engine.rs`: default template sets `connected_services: None` (filled in by `LocalService` per-session). Conservative default.

**Filter logic rewrite (`filter_tools_for_context`):**
- New signature: takes `connected_services: &[String]` (in addition to existing args).
- New capability-driven branch BEFORE the keyword scan: inserts `Calendar`/`Gmail`/`Drive`/`Slack`/`Atlassian`/`GitHub` into the active set iff the matching service name is in `connected_services`.
- Service-name mapping: `Calendar → "google_calendar"`, `Gmail → "gmail"`, `Drive → "google_drive"`, `Slack → "slack"`, `Atlassian → "atlassian"`, `GitHub → "github"`. Verified against the `SERVICE_NAME` consts in each integration's `integration.rs`.
- Dropped `github` and `google` from the `Web` keyword set (those were proxies for integration tools now gated by capability).
- Call site updated to query `connected_services` from `PromptContext` and pass it through. Defaults to empty `Vec` if no provider set.

**Unit tests added (4 new):**
- `calendar_tool_visible_when_calendar_capability_present_no_keywords` — message contains zero calendar/web keywords; `google_calendar` capability set; assert `calendar_upcoming` is visible.
- `calendar_tool_hidden_when_calendar_capability_absent` — same setup, empty capability set; assert dropped.
- `slack_tool_visible_when_slack_capability_present_no_keywords` — analogous positive for Slack.
- `slack_tool_hidden_when_slack_capability_absent` — analogous negative for Slack.

**Test infrastructure:** added `CategorizedStub` helper in the test mod — a configurable `Tool` impl with a per-instance category. Avoids dragging real integration crates into the engine's test deps.

**Validation:**
- `cargo check --workspace`: ✅ clean.
- `cargo build --workspace --release`: ✅ clean (1m 03s).
- `cargo test --workspace --lib`: ✅ **1,762 tests pass** (1,758 + 4 new), 0 fail.

T-0394's structural failure mode is now blocked: calendar tools survive iter-2+ unconditionally when `google_calendar` is connected, regardless of user message text.