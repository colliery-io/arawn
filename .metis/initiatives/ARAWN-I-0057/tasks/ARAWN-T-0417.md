---
id: t-a-template-params-cursor-types
level: task
title: "T-A: Template + params/cursor types for filesystem feed"
short_code: "ARAWN-T-0417"
created_at: 2026-05-25T15:47:48.851019+00:00
updated_at: 2026-05-26T01:34:34.038915+00:00
parent: ARAWN-I-0057
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0057
---

# T-A: Template + params/cursor types for filesystem feed

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0057]]

## Objective

Land the skeleton of the filesystem feed template: param + cursor types, validation, registration metadata. **No actual scanning yet — that's T-B.** Goal is a `FilesystemFeedTemplate` that compiles, registers, and rejects bad params before T-B touches the scan logic.

## Files

- **New:** `crates/arawn-feeds/src/clients/filesystem.rs` — template impl + param/cursor types
- **Modify:** `crates/arawn-feeds/src/clients/mod.rs` — `pub mod filesystem;`
- **Modify:** `crates/arawn-feeds/src/lib.rs` — re-export if/as appropriate

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

- [ ] `FilesystemFeedParams { root: PathBuf, recursive: bool, include: Vec<String>, exclude: Vec<String> }` defined with serde Deserialize/Serialize + `Default` impl producing the documented defaults (`recursive=true`, `include=["**/*"]`, six-entry exclude list)
- [ ] `FilesystemFeedCursor { files: BTreeMap<PathBuf, FileFingerprint> }` with `FileFingerprint { mtime: i64, size: u64 }`, both serde-round-tripping
- [ ] `FilesystemFeedTemplate` impl of `FeedTemplate`:
  - `name() == "filesystem/folder"`
  - `validate()` rejects: relative root, non-existent root, non-directory root, root with fewer than 3 path components, malformed glob in include or exclude
  - `defaults()` returns a 30s cadence
  - `run()` returns `unimplemented!()` for now (T-B fills it)
- [ ] Unit tests in the same file (inline `#[cfg(test)] mod tests`): one positive validate case, one negative case per rejection class
- [ ] `cargo check -p arawn-feeds` clean
- [ ] `cargo test -p arawn-feeds` passes the new module's tests

## Notes

- Reference existing templates for the shape: `crates/arawn-feeds/src/clients/slack.rs` or `gmail.rs` are the cleanest examples
- `globset` is already in the workspace; pull it via `globset = { workspace = true }` if it isn't already a direct dep of arawn-feeds
- Don't add `walkdir` yet — T-B owns that
- `BTreeMap` not `HashMap` so the cursor serializes deterministically

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

**2026-05-25 — Done.** Landed `crates/arawn-feeds/src/clients/filesystem.rs` with `FilesystemFeedParams`, `FilesystemFeedCursor`, `FileFingerprint`, and a `FilesystemFeedTemplate` impl of `FeedTemplate`:
- `name() == "filesystem/folder"`, `run()` is `unimplemented!()` (T-B owns it).
- `validate()` parses params then rejects, in order: relative root → root < 3 path components → non-existent/inaccessible root → non-directory root → malformed include/exclude glob.
- `defaults()` returns cadence + empty `{"files":{}}` cursor.
- Wired `pub mod filesystem;` + re-exports through `clients/mod.rs` and `lib.rs`.
- Promoted `globset = "0.4"` (already in lockfile via globwalk) to a direct workspace dep and added it to arawn-feeds. Did **not** add walkdir — T-B owns that.
- 12 inline unit tests, all green (`cargo test -p arawn-feeds filesystem`). Workspace compiles clean; remaining warnings are pre-existing in unrelated files.

**⚠ Cadence design conflict (flagged for human):** The initiative notes call for ~30s scan cadence/latency. The runtime enforces a **15-minute cadence floor** (`cadence::MIN_CADENCE`) and cron is minute-granularity, so a sub-minute cadence is **not representable** and would be rejected at registration. I defaulted to the floor (`*/15 * * * *`) and documented it in `DEFAULT_CADENCE`. True near-real-time watching would need a push/webhook surface (out of scope). 15-min is fine for transcript drops.