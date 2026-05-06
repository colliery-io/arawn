---
id: google-drive-integration-read
level: task
title: "Google Drive integration — read + write (search, list, read, upload, update, delete)"
short_code: "ARAWN-T-0205"
created_at: 2026-05-06T02:06:34.650400+00:00
updated_at: 2026-05-06T02:08:04.289994+00:00
parent: ARAWN-I-0033
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: ARAWN-I-0033
---

# Google Drive integration — read + write (search, list, read, upload, update, delete)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0033]]

## Objective

Add Google Drive as a fourth external integration alongside Gmail, Calendar, and Slack. Read + write tool surface so the agent can search/list/read/upload/update/delete files in the user's Drive. Same architectural pattern as T-0202 (Gmail) / T-0203 (Calendar): single shared `ARAWN_GOOGLE_*` OAuth client, reuses `google_common::ArawnGetToken` adapter, tools sit under `crates/arawn-integrations/src/drive/`.

User picked v1+v2 together (read + write) up front — explicit "I know this will be useful, take it to v2 now."

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

## Type / Priority
- Feature
- P1 — Drive is a major information surface for personal-assistant use ("read my notes folder", "drop today's meeting summary into Drive").

## Acceptance Criteria

## Acceptance Criteria

- [ ] `crates/arawn-integrations/src/drive/` with `integration.rs` + `tools.rs`. Mirrors Gmail/Calendar layout.
- [ ] `google-drive3` v7 dep added (gold-standard family).
- [ ] OAuth scope: `https://www.googleapis.com/auth/drive` (full read+write). Acknowledged tradeoff: more powerful than `drive.readonly` but matches the v2 write requirement.
- [ ] Tools registered:
  - `drive_search({query, page_size?})` — Google's query syntax (`name contains 'foo'`, `mimeType = 'application/pdf'`, `modifiedTime > '...'`). Returns `[{id, name, mime_type, size, modified_time, web_view_link}]`. Permission: ReadOnly.
  - `drive_list({folder_id?, page_size?})` — list root or folder contents. Same response shape as search. Permission: ReadOnly.
  - `drive_get_metadata({file_id})` — full metadata: name, mime_type, size, owners, modified_time, parents, web_view_link. Permission: ReadOnly.
  - `drive_read({file_id})` — file content. Google Docs/Sheets/Slides → export via text/csv/markdown; binary types → download bytes returned base64. Tool dispatches on mime_type. Permission: ReadOnly.
  - `drive_upload({name, content, mime_type, parent_folder_id?})` — create new file. Permission: Other (mode default: ask).
  - `drive_update({file_id, content})` — overwrite content (preserves metadata). Permission: Other (mode default: ask).
  - `drive_delete({file_id})` — moves to trash (Drive's standard recoverable delete; not permadelete). Permission: FileWrite.
- [ ] `ARAWN_GDRIVE_CLIENT_ID` / `_SECRET` env vars with `ARAWN_GOOGLE_CLIENT_ID` / `_SECRET` fallback. Same pattern as Gmail/Calendar.
- [ ] `[integrations.drive]` in `arawn.toml` falling back to `[integrations.google]`.
- [ ] `docs/src/integrations/drive.md` covers Cloud Console "Enable Drive API" + scope explanation + tool descriptions + `/connect google_drive` flow.
- [ ] Tests: parameter parsing per tool, response-shape mappers (DriveFile → summary), and one integration test stub against a recorded fixture (or `#[ignore]` placeholder per the deferred-corpus convention from T-0202/T-0203).

## Implementation Notes

- **Service name:** `google_drive` (snake_case, matches `google_calendar`).
- **Mime type dispatch in `drive_read`:** Google's export endpoint takes a target format. Sensible defaults:
  - `application/vnd.google-apps.document` → export as `text/markdown`
  - `application/vnd.google-apps.spreadsheet` → export as `text/csv`
  - `application/vnd.google-apps.presentation` → export as `text/plain`
  - Other (PDFs, plain text, images) → download raw bytes
  - For binary, return base64 + `mime_type` so the LLM knows what it got. Set a max-size cap (default 1 MB; configurable) — Drive will return arbitrarily large files and we don't want to dump 50 MB into a tool result.
- **Pagination:** Drive API returns a `nextPageToken`. Tools accept `page_size` (default 50, max 100) and surface `next_page_token` in the response when there's more. Agent can re-call with `page_token` to continue.
- **Default folder:** `drive_list` with no `folder_id` lists the user's root.
- **Capabilities summary:** `Integration::capabilities_summary` already returns `Option<String>` — Drive impl can return "google_drive (connected; can search, list, read, upload, update, delete files)".
- **Permission category for write ops:** matches Gmail's send pattern — `Other` (mode default: ask) for `upload`/`update`, `FileWrite` for `delete` (matches Gmail mark-read).
- **Scope check at execute time:** read tools work with `drive.readonly` if granted (fall through gracefully); write tools require `drive`. Per-tool required-scope declarations like the Slack pattern from T-0204 phase 1.
- **No dual-token model needed:** unlike Slack, Google Drive bot/user distinction doesn't apply — a single OAuth grant authorizes the user's Drive directly.

## Out of Scope (defer)

- File sharing / permission management (`drive.permissions` API). Real surface, separate task.
- Drive comments / replies. Niche for v1.
- Folder operations (create folder, move file). Mostly composable from upload+update; revisit if needed.

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

### 2026-05-06 — Implemented (7 tools), awaiting UAT

**Crate:** `google-drive3 = "7"` (gold-standard family, same as gmail1/calendar3) plus `http-body-util` for reading `Response<BoxBody>` bodies on the export/download paths.

**Crate structure** (`crates/arawn-integrations/src/drive/`):
- `integration.rs` — `GoogleDriveIntegration`, `GoogleDriveProviderConfig`, single full-write scope `https://www.googleapis.com/auth/drive`. Same OAuth shape as Calendar (single Google scope, single token).
- `client.rs` — `DriveHub` typed client built via shared `google_common::ArawnGetToken` adapter.
- `tools.rs` — seven `arawn_tool::Tool` impls plus a `FileSummary` projection mapping `DriveFile` → compact JSON.

**Tools registered:**

| Tool | Permission | Backed by |
|---|---|---|
| `drive_search` | ReadOnly | `files.list` (Drive query syntax) |
| `drive_list` | ReadOnly | `files.list` (parent-folder query) |
| `drive_get_metadata` | ReadOnly | `files.get` |
| `drive_read` | ReadOnly | `files.export` for Google natives, `files.get?alt=media` otherwise |
| `drive_upload` | Other (mode default: ask) | `files.create` (multipart) |
| `drive_update` | Other (mode default: ask) | `files.update` (multipart, preserves metadata) |
| `drive_delete` | FileWrite | `files.update` with `{trashed: true}` (NOT `files.delete` — that's permadelete; deliberately not exposed) |

**Read content dispatch** (`drive_read`):
- `application/vnd.google-apps.document` → exported as `text/markdown`
- `application/vnd.google-apps.spreadsheet` → `text/csv`
- `application/vnd.google-apps.presentation` → `text/plain`
- `application/vnd.google-apps.drawing` → `image/png`
- Forms / sites / scripts → clean error: "open in browser, no machine-readable export"
- Non-Google types → raw download via `alt=media`. Text-like (`text/*`, `application/json`, `application/xml`) decoded as UTF-8; binary returned base64. Capped at 1 MB default, 5 MB max.

**Capabilities summary:** integrates with the dynamic system-prompt fragment from T-0204 phase 2. Returns `"google_drive (connected; can search, list, read, upload, update, delete files)"` when connected.

**Wiring** (`crates/arawn/src/main.rs`):
- `ARAWN_GDRIVE_CLIENT_ID` / `_SECRET` env first, falls back to `ARAWN_GOOGLE_CLIENT_ID` / `_SECRET`.
- `[integrations.drive]` config block falls back to `[integrations.google]`. Same precedence as the other Google integrations.

**Tests** (5 new in `arawn-integrations`):
- `drive::tools::tests::export_mime_dispatch_covers_known_google_types`
- `drive::tools::tests::summarize_file_extracts_owner_emails`
- `drive::tools::tests::summarize_file_includes_parents_when_requested`
- `drive::integration::tests::default_provider_has_drive_scope`
- `drive::integration::tests::provider_lifts_into_oauth_config`

All 31 arawn-integrations tests pass; 0 clippy warnings after fixing two `field_reassign_with_default` lints in `drive_upload` / `drive_delete`.

### UAT — to run (next session)

**Prerequisites:**
1. Cloud Console → APIs & Services → Library → enable **Google Drive API** in the existing arawn project.
2. Restart the server (or it's already running with creds; check `/integrations`).
3. In TUI: `/connect google_drive`. OAuth dance — grant the `drive` scope.

**Suggested UAT prompts (in this order):**

| # | Prompt | Tool | Watch for |
|---|---|---|---|
| 1 | "list the files in my Drive root" | `drive_list` | Returns FileSummary array; `next_page_token` if > 50 |
| 2 | "search my Drive for files modified in the last week" | `drive_search` (query: `modifiedTime > '2026-04-29T00:00:00'`) | Agent constructs the right Drive query syntax |
| 3 | "show me metadata for file [pick one from the list]" | `drive_get_metadata` | Full metadata including `parents`, `owners` |
| 4 | "read [a small text file or Google Doc] from my drive" | `drive_read` | Google Docs export to markdown; plain files UTF-8 decode; binary base64 |
| 5 | "create a markdown file in my drive called 'arawn-test.md' with content 'hello from arawn'" | `drive_upload` | Permission prompt fires (Other category, mode default: ask). Verify in Drive UI. |
| 6 | "append a second line to that file" | `drive_get_metadata` to find id, then `drive_update` | Permission prompt. Content fully overwritten (not appended — agent should read+concat+update). |
| 7 | "delete the arawn-test.md file" | `drive_delete` | Permission prompt. File goes to Drive trash (recoverable). |
| 8 | (optional) "list the contents of [a folder you know has files]" | `drive_list` with `folder_id` | Folder-scoped list works |

**Things to watch for:**
- Tool descriptions visible to the agent include the Drive scope info (consistent with Slack pattern, though Drive tools don't yet do per-tool scope checks like Slack does).
- `drive_read` cap: try a known >1MB file; should return `truncated: true` and capped content.
- Permission prompts fire on uploads/updates/deletes (not silently auto-allowed in default mode).
- Capabilities summary line in agent output mentions Drive after connect; disappears after `/disconnect google_drive`.
- The agent should **not** suggest cron/shell workarounds for scheduled Drive work — that path was filed as a follow-up scaffolding gap in I-0033 followups.

If anything errors, paste the message — most likely failures are mime-type dispatch surprises on specific Google native types we didn't test (jamboard, fusion tables, etc.) or surprises on very large files past the cap.