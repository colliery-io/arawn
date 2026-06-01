---
id: remote-company-tracking
level: task
title: "Remote-company tracking observability plan (org_snapshot / awaiting_me / voice_check)"
short_code: "ARAWN-T-0467"
created_at: 2026-06-01T02:31:38.756770+00:00
updated_at: 2026-06-01T02:31:38.756770+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Remote-company tracking observability plan (org_snapshot / awaiting_me / voice_check)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

Captured plan for a future "remote-company tracking" surface — the strategic shift the user surfaced mid-I-0065: *"the major goal is this helps me keep track of a remote company."*

Three proposed tools sit at the heart of this. None built yet.

### `org_snapshot` — "what's happening across my org right now"
- Pulls recent activity per direct's team: PR throughput, calendar density (meeting load), slack channel volume in channels they own, recent incidents
- Daily-ish run; agent-callable on demand
- Substrate: PersonProfile.relation_to_user=Manages (the directs), calendar_events projection, slack/gmail/github projections (when GitHub tools land — see I-0066 deferral)
- Output: per-team rollup with deltas vs prior period

### `awaiting_me` — "what is the org blocked on me for"
- Inverse of inbox triage: items where YOUR decision/approval/feedback unblocks others
- Cross-source: gmail "waiting on you" threads, slack mentions in channels you own, calendar invites with unanswered RSVP, lens-binding requests, /code-review pending
- Output: sorted by urgency × blast-radius

### `voice_check` — "who haven't I heard from this week"
- Directs and skip-levels whose signal volume dropped relative to their baseline
- The remote-company canary — surfaces the people who tend to disappear before they actually disappear
- Substrate: per-person signal frequency from slack/gmail/github projections over rolling windows
- Output: list sorted by signal-volume delta, with last-seen timestamp

## Why this is in the backlog

User chose to file this rather than build now. The pivot is real but not urgent — the org-substrate (I-0064) and the read-tools (I-0065 T-A `person_brief` + T-B `cadence_check`) already cover meaningful daily-driver value. The three observability tools above are the next conceptually-coherent layer when ready.

## When to pick this up

- After GitHub team-level tools land (I-0066) — org_snapshot needs PR throughput visibility
- OR when the maintainer's day actually demands one of the three tools faster than the others would land — pick that one off and ship it as a standalone task

## Related history

- I-0065 closed with T-A (person_brief) + T-B (cadence_check) shipped on `feat/i0065-exec-surface`
- T-C (ExecComms voice), T-E (weekly audience knob) — dropped, not relevant to remote-company tracking
- T-D (/remember classifier upgrade), T-F (inbox/slack triage + VIP) — closed-as-deferred; remain candidates if needed individually later

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

*To be added during implementation*