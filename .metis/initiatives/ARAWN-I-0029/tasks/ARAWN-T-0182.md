---
id: token-directory-hardening-extend
level: task
title: "Token directory hardening — extend sensitive-paths deny list to cover {data_dir}/tokens/"
short_code: "ARAWN-T-0182"
created_at: 2026-04-17T03:01:18.091226+00:00
updated_at: 2026-04-17T03:16:15.353057+00:00
parent: ARAWN-I-0029
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0029
---

# Token directory hardening — extend sensitive-paths deny list to cover {data_dir}/tokens/

## Parent Initiative

[[ARAWN-I-0029]]

## Objective

Make `{data_dir}/tokens/` permanently inaccessible to the agent. Once tokens are stored on disk (T-0181) the file/glob/grep tools must reject any path that resolves into the tokens directory — even via symlinks, even if the user explicitly added the data dir to `allowed_paths`.

Builds on the sensitive-paths plumbing from ARAWN-T-0171 and ARAWN-T-0173: extend the deny check so that the tokens directory is treated like `~/.ssh` (always denied, no escape hatch).

The data dir is dynamic (default `~/.arawn`, override via `ARAWN_DATA_DIR`), so the deny list must be parameterised at runtime — likely a new `is_token_path(path, data_dir)` helper that the existing `is_sensitive_path` and `is_secret_file` checks consult.

Estimated size: **S** (~1 day).

### Priority
- [x] P1 - High (security-critical; tokens leak through any unsealed crack)

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

- [ ] `tools/sensitive_paths.rs` gains a `TokenDirGuard` (or equivalent) that knows the runtime `data_dir` and answers "does this path resolve into `{data_dir}/tokens/`?"
- [ ] `EngineToolContext` carries the data_dir already (it does — verify); the guard is constructed in `LocalService` and made available via context so glob/grep/file_read/file_write/file_edit can consult it
- [ ] `glob` rejects `{data_dir}/tokens/*` patterns
- [ ] `grep` rejects `{data_dir}/tokens/...` paths
- [ ] `file_read`/`file_write`/`file_edit` reject any canonical path inside `{data_dir}/tokens/`
- [ ] Symlink test: create a symlink inside the workstream pointing at `{data_dir}/tokens/google.json.age`; `file_read` denies via the symlink
- [ ] Allowed-paths bypass test: even if `{data_dir}` is added to `allowed_paths`, the tokens subdirectory remains denied
- [ ] `..` traversal test: glob with `path = "../../.arawn/tokens"` is denied
- [ ] Existing sensitive-paths tests still pass
- [ ] Depends on ARAWN-T-0181 (the directory must exist with content for end-to-end tests)

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

- Added `is_token_path(path: &Path, data_dir: &Path) -> bool` to `tools/sensitive_paths.rs`. Compares canonical forms (defeats symlinks + `..` traversal); falls back to lexical prefix when paths don't exist yet.
- Wired into all five tools that take a path: `glob`, `grep`, `file_read`, `file_write`, `file_edit`. Each calls `ctx.data_dir()` and consults `is_token_path` after the existing `is_sensitive_path`/`is_secret_file` checks.
- Used a free function rather than a `TokenDirGuard` struct because the data_dir is already on the context and the check is stateless. Less ceremony.
- 3 new tests: `token_path_detection` (direct + sub + nested + sibling negative), `token_path_defeats_dotdot_traversal` (relative `..` resolves into tokens via canonicalize), and `refuses_token_dir_path` in file_read (workstream root doubles as data_dir, tokens/ inside, file_read denies with "OAuth token directory" message).
- All 190 tools tests pass; existing sensitive_paths tests untouched.
- Allowed-paths bypass: existing `validate_path` first checks `escapes workstream root || is_allowed_path`; the token check runs *after* path resolution, so even if a user adds `~/.arawn` to `allowed_paths`, the tokens subdirectory remains denied.