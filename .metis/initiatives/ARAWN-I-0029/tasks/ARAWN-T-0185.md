---
id: integrationregistry-localservice
level: task
title: "IntegrationRegistry + LocalService wiring with stub providers (MissingCapability)"
short_code: "ARAWN-T-0185"
created_at: 2026-04-17T03:01:22.531934+00:00
updated_at: 2026-04-17T03:23:02.504729+00:00
parent: ARAWN-I-0029
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0029
---

# IntegrationRegistry + LocalService wiring with stub providers (MissingCapability)

## Parent Initiative

[[ARAWN-I-0029]]

## Objective

Wire the `IntegrationRegistry` into `LocalService` end-to-end. Until I-0030 and I-0031 land real providers, register **stub providers** that return `IntegrationError::MissingCapability` from every method. Proves the wiring works without faking external APIs.

Steps:
- Add `IntegrationRegistry` in `arawn-integration` (likely in `lib.rs` or its own module).
- Construct it in `main.rs` from `&ArawnConfig` after the `LlmClientPool` (so similar shape and ordering).
- Pass `Arc<IntegrationRegistry>` into `LocalService::new`.
- Stub provider impls live behind a feature/cfg or in `arawn-integration/src/providers/stub.rs` so they're not in the production hot path beyond the early-return error.

Estimated size: **S–M** (~1–2 days).

### Priority
- [x] P2 - Medium (closes the loop on the foundation)

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement  
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `IntegrationRegistry` struct with: `task_list: Option<Arc<dyn TaskListProvider>>`, `push: Option<Arc<dyn PushProvider>>`, `schedule: Option<Arc<dyn ScheduleProvider>>`, `messaging: Option<Arc<dyn MessagingProvider>>`
- [ ] `IntegrationRegistry::from_config(&ArawnConfig, ...) -> Self` — reads `[capabilities]`, looks up provider names, instantiates stub providers for now
- [ ] Stub providers (`StubTaskList`, `StubPush`, `StubSchedule`, `StubMessaging`) that return `IntegrationError::MissingCapability` from every method
- [ ] `LocalService::new` signature gains `integrations: Arc<IntegrationRegistry>` (mirrors `llm_pool` plumbing)
- [ ] `LocalService::shared_integrations()` accessor for tools that will need it
- [ ] `main.rs` builds the registry and passes it into `LocalService::new`
- [ ] Test fixture in `arawn-tests/tests/local_service.rs` uses `IntegrationRegistry::empty()` (a no-providers helper)
- [ ] Test: `IntegrationRegistry::from_config` with a `[capabilities]` block referring to a provider with a stub returns a registry where the matching capability is `Some`
- [ ] Test: a stub provider's method returns `IntegrationError::MissingCapability` with a clear message naming the provider+capability
- [ ] All existing workspace tests still pass
- [ ] Depends on ARAWN-T-0179, ARAWN-T-0183

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates

- New `arawn-integration::providers::stub::StubProvider` implements all four capability traits and returns `IntegrationError::MissingCapability { capability: "<slot> (provider '<name>' is not implemented in this build)" }` from every method.
- New `arawn-integration::registry` module with `IntegrationRegistry { task_list, push, schedule, messaging: Option<Arc<dyn ...>> }`, `CapabilityMap` (config-decoupled DTO), `ProviderLookup` trait, `from_capability_map(&CapabilityMap, &impl ProviderLookup)`, and `empty()` constructor.
- `ArawnConfig` now `impl arawn_integration::ProviderLookup` (returns `IntegrationProviderConfig::provider`); also gained `capability_map() -> CapabilityMap` projector.
- `LocalService::new` signature gains `integrations: Arc<IntegrationRegistry>`. New `shared_integrations()` accessor exposes it. Field stored alongside `llm_pool`.
- `main.rs` builds the registry from `&config.capability_map()` + `&config` (as the lookup), logs which slots are populated, and passes it into `LocalService::new`.
- Updated test fixtures in `arawn-tests/local_service.rs` and `websocket.rs` to pass `Arc::new(IntegrationRegistry::empty())`. Added `arawn-integration` to `arawn-tests` Cargo deps.
- 4 new registry tests: empty has no caps; configured slots get stubs; unknown-provider stays None; stub returns `MissingCapability` containing both slot and provider name.
- All workspace tests pass; integration crate now at 26 tests, arawn-tests local_service at 16, websocket at 14.