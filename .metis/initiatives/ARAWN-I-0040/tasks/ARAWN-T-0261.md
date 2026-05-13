---
id: uat-scenario-for-i-0040-file-based
level: task
title: "UAT scenario for I-0040 — file-based synthetic fixtures + signal-extraction-e2e"
short_code: "ARAWN-T-0261"
created_at: 2026-05-13T13:13:57.267974+00:00
updated_at: 2026-05-13T13:14:02.370472+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# UAT scenario for I-0040 — file-based synthetic fixtures + signal-extraction-e2e

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0040]]

## Objective **[REQUIRED]**

{Clear statement of what this task accomplishes}

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

## Acceptance Criteria **[REQUIRED]**

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

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

### 2026-05-13 — Scaffolding + first scenario in place

**Design pivot mid-task:** I initially proposed bypassing the extractor at seed time and dropping pre-extracted entities directly into each workstream's KB. User pushed back hard — the extractor *is* part of the system being evaluated, not infrastructure to skip. Reverted that direction; the seed path now runs the LLM extractor synchronously per (workstream, feed_type) before the server starts, so the agent's first turn sees a KB that came out of the real pipeline.

**File-based fixture format (`tests/fixtures/uat/*.json`):**
- Top-level `{ workstreams: [{ name, description, rows: [...] }] }`.
- Rows are a discriminated union by `feed_type` (`gmail_messages`, `slack_messages` today; additive variants for drive/jira/confluence/calendar later).
- Field shapes track the typed `Projection` structs in `arawn-projections` so a future "dump real feed → fixture" CLI can emit the same format.

**Fixture loader (`tests/uat_fixture.rs`):**
- `load(path) → Fixture`, `apply(&fixture, data_dir) → Applied` opens `Store` + `ProjectionStore`, materializes each workstream, batches rows through the typed `Projection::row()` writers.
- `build_seed_llm_client(provider, model, api_key_env)` mirrors `arawn-bin::build_llm_client` so the seed-time extractor uses the same provider config the server writes into `arawn.toml`.
- `drive_extraction(applied, data_dir, client, model, cap)` instantiates a `CotChain` + `ExtractorRunner` and calls `run_for_workstream_until_exhausted` per (workstream, feed_type). Returns total rows processed.

**UAT harness integration (`tests/uat.rs`):**
- `Scenario` grew an optional `seed_fixture: Option<String>` field (relative to `CARGO_MANIFEST_DIR`).
- `uat_run` loads + applies the fixture, builds the seed client, drives extraction with a 15-minute wall-clock cap, *then* starts the arawn server. Existing `github-monitor` and `work-signal-pipeline` scenarios set `seed_fixture: None` and behave exactly as before.

**Synthetic fixture (`tests/fixtures/uat/signal-extraction-e2e.json`):**
- Two workstreams (`work`, `dnd`); 14 + 12 projection rows across gmail + slack.
- Deliberate noise embedded for downstream tools to react to:
  - Two near-duplicate Postgres rows for re-shelve.
  - An old "falcon" cluster (3 slack rows from January, far past the dust idle threshold) for the manual dust scenario.
  - A cross-workstream identity tease: "Alice Chen" in `work` and "Alice Chen ... different from Alice the Bard" in `dnd` for door-watch to potentially flag.
  - Off-topic items (cafeteria menu, coffee machine, random TV chat) the classify stage should skip.
  - A late "should we reconsider mysql" message that reaffirmation logic could supersede or counter.

**Scenario (`signal-extraction-e2e`):**
- 8 turns exercising `workstream_switch`, `signal_search`, `signal_query`, `signal_timeline`, `workstream_dust`, `workstream_refine`, `workstream_apply`, `workstream_rollback`.
- Mechanical threshold: `min_memory_entities ≥ 6` (cheap floor; UAT will tell us where to set this realistically).
- Excluded from default `all_scenarios()` runs only via `UAT_SCENARIO=signal-extraction-e2e` filter — keeping LLM-cost discoverability explicit until we know what reliable thresholds look like.

**Tests:** fixture loader has 4 inline tests + a smoke test that loads the actual signal-extraction-e2e.json — all passing. Workspace + clippy green.

**Next step:** actually *run* the scenario. That happens on the user's side via `angreal test uat --scenario signal-extraction-e2e` (or env-var-equivalent). Expect to surface real behaviors: prompt phrasing the agent doesn't follow, tool routing mistakes, mechanical thresholds that need tuning, fixture rows that need more variation. Findings drive a follow-up task or amend this one.