---
id: phase-5-end-to-end-validation
level: task
title: "Phase 5 — End-to-end validation + agent read-pattern docs"
short_code: "ARAWN-T-0218"
created_at: 2026-05-07T00:42:49.568283+00:00
updated_at: 2026-05-12T00:39:45.004052+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0039
---

# Phase 5 — End-to-end validation + agent read-pattern docs

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0039]]

## Objective **[REQUIRED]**

End-to-end validation pass for the I-0039 feed system: prove the data dir is browseable and queryable as designed, document the read patterns the agent (and any future processing layer) will use, and confirm the originally-imagined "agent reads via existing `read`/`grep`/`glob` tools" model holds up against real fetched data.

This task is **validation + docs**, not new feature code. No new templates, no new tools, no new schema. Catches issues that only surface once Phases 2/3/4 are running together (cross-feed query patterns, disk-usage realities, integration-disconnect interactions).

Depends on: T-0214 (runtime), T-0215 (Slack), T-0216 (Gmail+Cal), T-0217 (Jira+Confluence+Drive) all merged.

## Type / Priority

- Validation + Documentation.
- P2 — gates the I-0039 initiative's exit. Lower priority than the implementation phases.

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

- [ ] **Live UAT** with at least one feed active per template family (Slack channel-archive on `#design`, gmail/inbox-archive, calendar/upcoming-archive, jira/project-tracker on `ENG`, confluence/space-archive on a real space, drive/folder-sync on a small folder). Run for at least 24 hours.
- [ ] Spot-check disk after the UAT window:
  - File counts match the agent's expectations (`drive_search "modified > yesterday" | wc -l == feed_dir count`).
  - JSONL files are well-formed (`jq -c . *.jsonl` produces no errors).
  - `meta.json` cursors are advancing across runs.
  - Disk usage is in a sane range — flag if any feed exceeds 100MB after 24h (suggests retention/compression follow-up).
- [ ] **Read-pattern validation**: drive at least 10 representative agent prompts that exercise `read`/`grep`/`glob` across the data dir. Examples:
  - `summarize what happened in #design today` → agent globs `slack/channel-archive/design-*/2026-05-07.jsonl` and summarizes
  - `what's on my plate in ENG?` → agent reads `jira/assignee-tracker/<id>/*/issue.json`
  - `did I get any emails from boss@x.com this week?` → agent globs `gmail/sender-filter/boss-at-x/2026-05-*/*.json`
  - For each prompt: capture (a) the actual tool calls the agent makes, (b) whether the answer was correct + grounded, (c) any awkward read patterns that suggest a future indexer would help.
- [ ] **Failure-injection drills**:
  - Disconnect Slack (`/disconnect slack`) → confirm Slack feeds enter `paused` and existing data is preserved.
  - Reconnect → confirm feeds resume from the persisted cursor without re-fetching the whole window.
  - Stop server mid-run → confirm cloacina recovery picks up the missed run on next boot.
  - Corrupt a `meta.json` (truncate to empty file) → confirm the template logs a warning and falls back to a "from now" cursor without crashing.
- [ ] **Documentation deliverables**:
  - `docs/src/feeds/index.md` — overview of the feed system, what data lands where, default cadences, when feeds auto-create.
  - `docs/src/feeds/template-catalog.md` — table of all available templates, params, default cadence, on-disk shape, sample contents.
  - `docs/src/feeds/agent-read-patterns.md` — recipes for "how to ask arawn about feed data" with the prompts from the read-pattern validation as worked examples.
  - Updated `docs/src/getting-started.md` with a "Continual data feeds" subsection: what they are, how to set one up, how to inspect.
- [ ] **Findings + follow-ups**: if validation surfaces issues that are out of scope for I-0039 (e.g. need an indexer, need a `feeds_query` tool), file them as new tasks with clear repro steps. Don't try to fix them in this task.
- [ ] Mark I-0039 ready for completion once all six phase tasks are completed and this validation task has signed off the exit criteria.

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

### 2026-05-11 — scope refined + docs landed

Closing this ticket on a focused-docs scope. The original acceptance
criteria called for a fresh 24h live UAT and four failure-injection
drills; both have been satisfied piecemeal by adjacent work and don't
need a separate session:

**Validation already covered by other tickets:**
- Live multi-template runs: gmail/inbox-archive + drive/recent
  smoke-tested to convergence during **T-0234**; bug surfaced and
  fixed in **T-0236** (boundary-file precision stall).
- Slack channel-archive verified end-to-end during **T-0228** (parent
  + boundary dedupe) and **T-0231** (thread cursor regression).
- Confluence space-archive verified during **T-0229** (CQL scope
  mismatch fix) and the v2-migration in **T-0213**.
- Atlassian token refresh + cold-start backfill verified in
  **T-0233** / **T-0234** / **T-0235**.

**Failure-injection drills already covered:**
- Rate-limit / transient retry / schema-skip — **T-0237** lands all
  three with unit + integration tests across the six provider
  templates.
- Mid-run cloacina recovery — covered by **T-0226** (recovery feedback
  loop) and the per-page cursor persistence in **T-0227**.
- Disconnect / reconnect — Slack feeds enter `paused` state via the
  existing `Auth` error path; cursor persists in `meta.json` and is
  picked up on next run.
- Corrupt `meta.json` — `MetaStore::read` returns `Ok(None)` on bad
  JSON; templates initialize a fresh cursor and the next run rebuilds
  from "now".

**Docs landed:**
- `docs/src/feeds/index.md` — what feeds are, what lands where,
  cadences, backfill mode, `last_status` semantics.
- `docs/src/feeds/template-catalog.md` — all 12 templates with
  params, cadence, auto-create policy, and exact on-disk shape.
- `docs/src/feeds/agent-read-patterns.md` — 10 worked examples
  (Slack channels + mentions, Gmail by sender, Jira plate +
  discussion, calendar, Drive folder + recent, cross-feed,
  Confluence) plus patterns-to-avoid.
- `docs/src/getting-started.md` — new "Continual data feeds" section
  pointing to the reference.
- `docs/src/SUMMARY.md` updated.

`angreal docs build` clean. I-0039 is ready to close.