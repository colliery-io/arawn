---
id: phase-4-jira-confluence-drive-feed
level: task
title: "Phase 4 — Jira + Confluence + Drive feed templates"
short_code: "ARAWN-T-0217"
created_at: 2026-05-07T00:42:44.849788+00:00
updated_at: 2026-05-07T00:42:44.849788+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0039
---

# Phase 4 — Jira + Confluence + Drive feed templates

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0039]]

## Objective **[REQUIRED]**

Implement Jira + Confluence + Drive feed templates per I-0039's Phase 4 plan. Five templates land:

- `jira/project-tracker` — issues + comments + history for a project. Param: `project` (key like `ENG`).
- `jira/assignee-tracker` — personal feed: `assignee = currentUser()`. Auto-created on `/connect atlassian`.
- `confluence/space-archive` — pages + bodies in a space. Param: `space_key`.
- `drive/folder-sync` — rsync-style mirror of a Drive folder, native files on disk. Param: `folder` (path or id).
- `drive/recent` — personal feed: files modified in the last N days. Auto-created on `/connect google_drive`.

Depends on: T-0214 (runtime), T-0213 (Atlassian v2 client landed already).

**Reference:** I-0039 Detailed Design; existing `arawn-integrations/src/{atlassian,drive}/` clients.

## Type / Priority

- Feature.
- P1 — completes the personal-feed defaults across all five integrations and adds the most useful watched-space patterns.

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

- [ ] All five templates registered.
- [ ] **Cursors**:
  - Jira: persist `updated >= last_seen_iso` JQL fragment + the highest `updated` timestamp seen in `meta.json`.
  - Confluence: persist `last_modified_iso` per space; v2 cursor pagination already works in our client from T-0213.
  - Drive: persist `pageToken` for changes-API style cursoring; `drive/recent` uses `modifiedTime > cursor`.
- [ ] **Disk layout**:
  - `jira/project-tracker/<feed_id>/<ISSUE-KEY>/issue.json` — overwrite per run (latest snapshot).
  - `jira/project-tracker/<feed_id>/<ISSUE-KEY>/comments.jsonl` — append-only.
  - `jira/project-tracker/<feed_id>/<ISSUE-KEY>/history.jsonl` — append-only.
  - `jira/assignee-tracker/<feed_id>/<ISSUE-KEY>/issue.json` (no comments/history for personal — keep it light).
  - `confluence/space-archive/<feed_id>/<page_id>/page.json` (metadata) + `confluence/space-archive/<feed_id>/<page_id>/body.storage.xml` (raw body).
  - `drive/folder-sync/<feed_id>/<original_path>` — native file body on disk; preserves Drive's folder structure.
  - `drive/recent/<feed_id>/<YYYY-MM-DD>/<file_id>.json` — metadata only (recent doesn't mirror bodies).
- [ ] `validate(params)`:
  - `project-tracker` — requires `project` non-empty; resolves to project ID at registration time.
  - `assignee-tracker` — no params.
  - `space-archive` — requires `space_key` non-empty.
  - `folder-sync` — requires `folder` non-empty; resolves to folder ID at registration time.
  - `recent` — no required params, optional `days_back: u32` (default 7).
- [ ] `defaults(params)`: cadence `30m` for jira/confluence, `1h` for drive/folder-sync, `30m` for drive/recent.
- [ ] **drive/folder-sync semantics**:
  - One-way pull only.
  - Preserves Drive's folder structure under `feed_dir/`.
  - Google native files (Docs/Sheets/Slides) exported per the existing `drive_read` mime-dispatch policy (Doc → markdown, Sheet → CSV, Slide → plain text).
  - Deleted files are deleted locally (mirror semantics, not append).
  - `meta.json` carries a per-file `etag/md5` map so we re-fetch only on change.
- [ ] Auto-create `jira/assignee-tracker` on `/connect atlassian` and `drive/recent` on `/connect google_drive`. Idempotent.
- [ ] **Failure modes**: token expired/scope removed → `FeedError::Auth`; rate-limit → `FeedError::RateLimited(retry_after)`; provider 410/deprecation surface → `FeedError::Schema(detail)`.
- [ ] **Tests** (in `arawn-feeds/src/templates/{jira,confluence,drive}/`):
  - `validate_rejects_missing_required_params` (per template).
  - `project_tracker_appends_new_comments_overwrites_issue_snapshot`.
  - `space_archive_writes_per_page_metadata_and_body`.
  - `folder_sync_mirrors_native_files_and_exports_google_natives`.
  - `cursor_advances_only_on_successful_persist`.
  - `auto_create_on_connect_is_idempotent`.
- [ ] `angreal check workspace` and `angreal check clippy` clean. All existing arawn-integrations tests still pass.

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