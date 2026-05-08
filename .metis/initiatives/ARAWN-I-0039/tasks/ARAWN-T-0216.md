---
id: phase-3-gmail-calendar-feed
level: task
title: "Phase 3 — Gmail + Calendar feed templates"
short_code: "ARAWN-T-0216"
created_at: 2026-05-07T00:42:27.570116+00:00
updated_at: 2026-05-07T00:42:27.570116+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0039
---

# Phase 3 — Gmail + Calendar feed templates

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0039]]

## Objective **[REQUIRED]**

Implement Gmail + Calendar feed templates per I-0039's Phase 3 plan. Four templates land:

- `gmail/inbox-archive` — personal feed: last N days of inbox, JSON-per-message. Auto-created on `/connect gmail`.
- `gmail/sender-filter` — watched-sender feed. Param: `sender_pattern` (full email or wildcard).
- `gmail/label-archive` — watched-label feed. Param: `label`.
- `calendar/upcoming-archive` — today + N days, one JSON per event. Auto-created on `/connect google_calendar`.

Depends on: T-0214 (runtime).

**Reference:** I-0039 Detailed Design; existing `arawn-integrations/src/{gmail,calendar}/` clients.

## Type / Priority

- Feature.
- P1 — calendar + inbox are the core "what's happening today" data sources for any briefing.

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

- [ ] All four templates registered.
- [ ] **Gmail cursor**: persist Gmail `historyId` in `meta.json`. On run, `users.history.list({startHistoryId: cursor})` for delta; on first run, fall back to `messages.list` with `q: newer_than:7d` (or label/sender query for the filtered templates).
- [ ] **Calendar cursor**: persist `updatedMin` ISO timestamp. Run pulls `events.list({timeMin: today, timeMax: today+N, updatedMin: cursor})` to capture both new and modified events.
- [ ] **Disk layout**:
  - `gmail/inbox-archive/<feed_id>/YYYY-MM-DD/<message_id>.json` (one file per email, raw API payload).
  - `gmail/sender-filter/<feed_id>/YYYY-MM-DD/<message_id>.json`.
  - `gmail/label-archive/<feed_id>/YYYY-MM-DD/<message_id>.json`.
  - `calendar/upcoming-archive/<feed_id>/<event_id>.json` — overwrite per run (events change; we keep latest snapshot per id, not append).
- [ ] `validate(params)`:
  - `inbox-archive` — no params required, optional `days_back: u32` (default 7).
  - `sender-filter` — requires `sender_pattern` non-empty.
  - `label-archive` — requires `label` non-empty; validates the label exists in the user's account at registration time.
  - `upcoming-archive` — no params required, optional `days_ahead: u32` (default 7).
- [ ] `defaults(params)`: cadence `15m` for inbox-archive, `30m` for sender-filter, `30m` for label-archive, `30m` for upcoming-archive.
- [ ] `run(ctx, params, feed_dir)` re-uses the existing google-* clients via the integration registry. No new auth/transport plumbing.
- [ ] Auto-create on `/connect gmail` (creates `gmail/inbox-archive`) and `/connect google_calendar` (creates `calendar/upcoming-archive`). Idempotent.
- [ ] **Failure modes**: token expired/scope removed → `FeedError::Auth`; rate-limit (429) → `FeedError::RateLimited(retry_after)`.
- [ ] **Tests** (in `arawn-feeds/src/templates/{gmail,calendar}/`):
  - `validate_rejects_missing_required_params` (per template)
  - `inbox_archive_writes_per_message_json_to_correct_path`
  - `cursor_advances_only_on_successful_persist`
  - `calendar_overwrites_existing_event_file_on_re_fetch` (proves the per-event overwrite semantics)
  - `auto_create_on_connect_is_idempotent`
- [ ] `angreal check workspace` and `angreal check clippy` clean. All existing tests still pass.

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