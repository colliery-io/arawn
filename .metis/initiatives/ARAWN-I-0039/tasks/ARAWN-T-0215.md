---
id: phase-2-slack-feed-templates
level: task
title: "Phase 2 — Slack feed templates (channel-archive, my-mentions, dm-archive)"
short_code: "ARAWN-T-0215"
created_at: 2026-05-07T00:42:23.559403+00:00
updated_at: 2026-05-07T00:42:23.559403+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0039
---

# Phase 2 — Slack feed templates (channel-archive, my-mentions, dm-archive)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0039]]

## Objective **[REQUIRED]**

Implement Slack feed templates per I-0039's Phase 2 plan. Three templates land:

- `slack/channel-archive` — archive a single Slack channel as JSONL, time-partitioned per day. Param: `channel` (channel name like `#design` or id like `CABCDEF`).
- `slack/my-mentions` — personal feed: all messages mentioning `@me` across the user's joined channels. Auto-created on `/connect slack`.
- `slack/dm-archive` — archive a 1-on-1 DM thread. Param: `user` (Slack user id or name).

Depends on: T-0214 (runtime + trait + cloacina wiring) merged.

**Reference:** I-0039's "Detailed Design" section for disk layout conventions; T-0214 for the `FeedTemplate` trait shape; existing `arawn-integrations/src/slack/` for the auth + client surface to call.

## Type / Priority

- Feature.
- P1 — Slack is the highest-volume, most-watched provider; this template family is what most users will configure first.

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

- [ ] All three templates registered in the `FeedTemplateRegistry` from T-0214 under names `slack/channel-archive`, `slack/my-mentions`, `slack/dm-archive`.
- [ ] **Cursor model**: each template persists Slack `latest_ts` in the feed's `meta.json`. Subsequent runs request `oldest = latest_ts` so we only fetch new messages.
- [ ] **Disk layout**:
  - `slack/channel-archive/<feed_id>/YYYY-MM-DD.jsonl` — append-only JSONL, one message per line, raw Slack API payload.
  - `slack/my-mentions/<feed_id>/YYYY-MM-DD.jsonl` — same shape; `feed_id` is `me-<workspace_id>`.
  - `slack/dm-archive/<feed_id>/YYYY-MM-DD.jsonl` — same shape.
- [ ] `validate(params)`: rejects missing/empty `channel` for channel-archive, missing `user` for dm-archive. `my-mentions` takes no params.
- [ ] `defaults(params)`: returns sensible cadence — `15m` (the floor) for channel-archive and my-mentions, `1h` for dm-archive.
- [ ] `run(ctx, params, feed_dir)`: re-uses the existing slack-morphism client from `arawn-integrations`. Channel-name-to-id resolution happens at validation time (so a deleted channel surfaces an error before the cron task fires).
- [ ] Auto-create `slack/my-mentions` feed on `/connect slack` success. Idempotent: if a feed already exists with the same template+default params for this workspace, do nothing.
- [ ] **Failure modes**: token expired/revoked → `FeedError::Auth`; rate-limited → `FeedError::RateLimited(retry_after)` honoring Slack's `Retry-After` header.
- [ ] **Tests** (in `arawn-feeds/src/templates/slack/`):
  - `validate_rejects_empty_channel`
  - `channel_archive_writes_jsonl_to_correct_path` (with a fake/recorded slack response)
  - `cursor_advances_only_on_successful_write` (drop the file write mid-run, verify cursor unchanged)
  - `my_mentions_auto_creates_on_connect` (wires through the integration registry)
- [ ] `angreal check workspace` and `angreal check clippy` clean. Existing arawn-integrations tests still pass.

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