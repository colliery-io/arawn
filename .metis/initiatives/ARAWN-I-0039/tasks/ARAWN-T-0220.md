---
id: slack-my-mentions-feed-template
level: task
title: "slack/my-mentions feed template + search.messages adapter"
short_code: "ARAWN-T-0220"
created_at: 2026-05-08T20:16:23.932087+00:00
updated_at: 2026-05-08T20:22:54.953397+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0039
---

# slack/my-mentions feed template + search.messages adapter

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0039]]

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

## Status Updates **[REQUIRED]**

### 2026-05-08 — Landed

- New trait methods on `SlackFeedClient`: `auth_test()` returning `SlackAuthInfo { user_id, team_id }`; `search_messages(query, oldest_ts)` returning `SlackHistoryPage`.
- `RealSlackClient::auth_test` via slack-morphism. `RealSlackClient::search_messages` via raw `reqwest` against `https://slack.com/api/search.messages` using the user token extracted from the existing `user_context()` path. Maps `missing_scope` / `not_authed` / `invalid_auth` / `ratelimited` to typed `FeedError` variants.
- `MyMentionsTemplate`:
  - Cursor `{ my_user_id, latest_ts }`, both nullable initially.
  - First run: cache miss → `auth_test` → store user_id in cursor. Subsequent runs: cache hit, no `auth_test` call.
  - Builds `<@U01ABC>` query, calls `search_messages` with prior cursor → Slack's `after:YYYY-MM-DD` filter.
  - Filters page by `ts > prior_latest_ts` to dedupe day-grained overlap. Writes to `<YYYY-MM-DD>.jsonl` keyed by message ts.
- New deps in `arawn-feeds`: `reqwest` (workspace), `rvstruct` (for accessing slack-morphism's `ValueStruct`-wrapped token).
- Existing mocks updated to satisfy the expanded trait via `unreachable!` stubs (channel/dm-archive don't exercise these methods).
- 4 integration tests in `tests/slack_my_mentions.rs` covering: first-run resolution, cached-id second run, day-grained overlap dedupe, empty-result no-op, and missing-integration auth error.

58 arawn-feeds tests green (38 unit + 3 cloacina-fire + 11 channel-archive + 2 dm-archive + 4 my-mentions). Workspace + clippy clean.