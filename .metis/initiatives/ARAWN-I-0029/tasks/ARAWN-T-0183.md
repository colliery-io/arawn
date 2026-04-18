---
id: config-plumbing-integrations-and
level: task
title: "Config plumbing — [integrations.*] and [capabilities] parsing in arawn.toml"
short_code: "ARAWN-T-0183"
created_at: 2026-04-17T03:01:19.622018+00:00
updated_at: 2026-04-17T03:19:04.628886+00:00
parent: ARAWN-I-0029
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0029
---

# Config plumbing — [integrations.*] and [capabilities] parsing in arawn.toml

## Parent Initiative

[[ARAWN-I-0029]]

## Objective

Add `[integrations.*]` and `[capabilities]` sections to `arawn.toml` parsing in `crates/arawn/src/config.rs`. Update `generate_default_toml()` to include commented examples for Google and Slack (matching the providers that I-0030 and I-0031 will land).

```toml
[integrations.google]
provider = "google"
client_id_env = "GOOGLE_CLIENT_ID"
client_secret_env = "GOOGLE_CLIENT_SECRET"
scopes = ["tasks", "calendar"]

[integrations.slack]
provider = "slack"
bot_token_env = "SLACK_BOT_TOKEN"
default_channel = "#arawn-notifications"

[capabilities]
task_list = "google"
schedule  = "google"
messaging = "slack"
push      = "slack"
```

Estimated size: **S** (~half day, mostly serde + tests).

### Priority
- [x] P3 - Low (no behaviour change until I-0030/I-0031, but unblocks both)

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

- [ ] `IntegrationProviderConfig { provider: String, client_id_env: Option<String>, client_secret_env: Option<String>, bot_token_env: Option<String>, scopes: Vec<String>, default_channel: Option<String> }` (or similar) — flexible enough to hold Google and Slack shapes without a separate type per provider
- [ ] `CapabilitiesConfig { task_list: Option<String>, push: Option<String>, schedule: Option<String>, messaging: Option<String> }`
- [ ] `ArawnConfig` gets `integrations: HashMap<String, IntegrationProviderConfig>` and `capabilities: CapabilitiesConfig`, both `#[serde(default)]`
- [ ] `generate_default_toml()` includes commented `[integrations.google]`, `[integrations.slack]`, and `[capabilities]` blocks with realistic values
- [ ] Test: parsing the documented multi-provider TOML produces the expected struct
- [ ] Test: omitting `[integrations]` and `[capabilities]` entirely still loads cleanly (defaults)
- [ ] Test: `generate_default_toml_is_parseable` still passes
- [ ] No code paths consume the new config yet — that's T-0185

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

- Added `IntegrationProviderConfig { provider, client_id_env, client_secret_env, bot_token_env, scopes, default_channel }` and `CapabilitiesConfig { task_list, push, schedule, messaging }` (all optional fields). Wired into `ArawnConfig` as `integrations: HashMap<String, _>` and `capabilities: _`, both `#[serde(default)]`.
- Updated `generate_default_toml()` with commented `[integrations.google]`, `[integrations.slack]`, `[capabilities]` blocks. Bumped raw-string delimiter from `r#"..."#` to `r##"..."##` because the example contains `"#arawn-notifications"` whose `"#` would otherwise terminate the raw string early. Same fix applied to the new test that parses the documented TOML.
- 2 new tests: `integrations_and_capabilities_parse_when_present` (full Google + Slack + capabilities round-trip) and `integrations_and_capabilities_default_to_empty` (omitted sections produce empty defaults). All 11 config tests pass; `generate_default_toml_is_parseable` still green.
- No code consumes the new config yet — wiring lands in T-0185.