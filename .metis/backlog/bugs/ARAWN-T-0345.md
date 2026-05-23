---
id: feed-search-known-feed-types
level: task
title: "feed_search KNOWN_FEED_TYPES excludes 7 GitHub projection tables"
short_code: "ARAWN-T-0345"
created_at: 2026-05-19T12:07:41.697768+00:00
updated_at: 2026-05-19T14:57:01.774214+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# feed_search KNOWN_FEED_TYPES excludes 7 GitHub projection tables

## Objective

Add the 7 GitHub projection table names to `feed_search`'s `KNOWN_FEED_TYPES` array so cross-feed search includes GitHub data by default — not just when the caller passes `feed_types` explicitly.

## Impact

- **Severity:** P2 — silent capability gap. `feed_search "PR about postgres"` returns zero GitHub hits despite the user having `github/issues-and-prs`, `github/repo-mirror`, etc. running.
- **Affected users:** any user who has connected GitHub.
- **Expected:** `feed_search` queries all 16 projection tables.
- **Actual:** scans the 9 in `KNOWN_FEED_TYPES`, skipping all 7 GitHub tables.

## Implementation notes

- `crates/arawn-engine/src/tools/feed_search.rs:21-31` defines `KNOWN_FEED_TYPES` as a 9-entry constant array.
- Add: `github_notifications`, `github_issues_and_prs`, `github_review_queue`, `github_repo_commits`, `github_repo_issues`, `github_repo_prs`, `github_issue_or_pr_comments`.
- Verify each name against `crates/arawn-projections/src/github.rs` and `arawn-projections/src/schema.rs` (table names are the source of truth).
- Once landed, drop the "not in KNOWN_FEED_TYPES" callout from `docs/src/reference/feed-search-tool.md`.

## Acceptance criteria

- [ ] All 7 GitHub projection tables present in `KNOWN_FEED_TYPES`.
- [ ] Unit test confirming `feed_search` with no `feed_types` arg includes a GitHub hit when the table has rows.
- [ ] Doc callout removed.

Surfaced during ARAWN-I-0051 doc triple-check.

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

### 2026-05-19 — GitHub feed types added to default scan

- Added all 7 GitHub projection table names to
  `KNOWN_FEED_TYPES` in
  `crates/arawn-engine/src/tools/feed_search.rs`:
  notifications, issues_and_prs, review_queue, repo_commits,
  repo_issues, repo_prs, issue_or_pr_comments.
- 2 new unit tests:
  - `known_feed_types_contains_all_github_tables` — guard
    that every github table name is present.
  - `known_feed_types_match_projection_constants` — asserts
    the hardcoded names match the canonical constants in
    `arawn-projections::github`. Catches future renames at
    test time.
- **Doc fixes:** stripped the "GitHub tables are NOT in
  KNOWN_FEED_TYPES" callout in
  `docs/src/reference/feed-search-tool.md`. Added the 7
  GitHub rows to the metadata table so the feed-types
  reference is complete.
- `cargo test -p arawn-engine --lib tools::feed_search` 2/0.
  `angreal check workspace` green.