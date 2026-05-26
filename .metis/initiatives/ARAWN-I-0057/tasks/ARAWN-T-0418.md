---
id: t-b-scan-and-diff-core-run-emits
level: task
title: "T-B: Scan-and-diff core — run() emits created/modified/deleted signals"
short_code: "ARAWN-T-0418"
created_at: 2026-05-25T15:47:52.628055+00:00
updated_at: 2026-05-26T01:38:30.008769+00:00
parent: ARAWN-I-0057
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0057
---

# T-B: Scan-and-diff core — run() emits created/modified/deleted signals

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0057]]

## Objective

Implement the actual scan-and-diff core. `run()` walks the watched root, builds a current file map, diffs it against the prior cursor's map, and writes one signal per changed file. Initial run (empty cursor) emits a `created` signal for every match — this is the "auto-index on first registration" behavior.

## Files

- **Modify:** `crates/arawn-feeds/src/clients/filesystem.rs` — replace T-A's `unimplemented!()` with the real `run()`
- **Add:** `walkdir = { workspace = true }` (or `walkdir = "2"`) as a direct dep of arawn-feeds if not already there

## Algorithm

```
let prev: BTreeMap<PathBuf, FileFingerprint> = parse(cursor).map_or(empty, |c| c.files);
let matcher = GlobSet::new(params.include) - GlobSet::new(params.exclude);
let walker = if params.recursive { WalkDir::new(root) } else { WalkDir::new(root).max_depth(1) };

let mut curr: BTreeMap<PathBuf, FileFingerprint> = BTreeMap::new();
for entry in walker {
    let entry = entry.ok()?;
    if !entry.file_type().is_file() { continue; }
    let rel = entry.path().strip_prefix(&root)?;
    if !matcher.matches(rel) { continue; }
    let meta = entry.metadata()?;
    curr.insert(entry.path().into(), FileFingerprint {
        mtime: meta.modified()?.duration_since(UNIX_EPOCH)?.as_secs() as i64,
        size: meta.len(),
    });
}

// Diff
for (path, fp) in &curr {
    match prev.get(path) {
        None => emit("created", path, Some(fp)),
        Some(p) if p != fp => emit("modified", path, Some(fp)),
        _ => (),  // unchanged
    }
}
for (path, _) in &prev {
    if !curr.contains_key(path) {
        emit("deleted", path, None);
    }
}

new_cursor = FilesystemFeedCursor { files: curr };
```


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

- [ ] `run()` writes one signal per changed file in the documented JSON shape (`source`, `path`, `rel_path`, `event`, `ts`, `size_bytes`, `mtime`)
- [ ] Initial run with empty cursor emits exactly N `created` signals where N = matching files in root
- [ ] Modifying one file → next `run()` emits exactly one `modified` signal
- [ ] Deleting one file → next `run()` emits exactly one `deleted` signal (with `size_bytes` and `mtime` as `null`)
- [ ] Unchanged files emit nothing
- [ ] `exclude` globs always win over `include` globs (e.g. `include=["**/*"]` + `exclude=["target/**"]` skips `target/`)
- [ ] `recursive=false` only walks the top level — files in subdirs are ignored
- [ ] Cursor round-trip: serialize-then-deserialize produces an equal `BTreeMap`
- [ ] Unit tests in inline `#[cfg(test)] mod tests` use `tempfile::tempdir`:
  - `empty_cursor_emits_created_for_each_match`
  - `unchanged_file_emits_nothing`
  - `modified_file_emits_modified`
  - `deleted_file_emits_deleted`
  - `exclude_glob_skips_matching_files`
  - `recursive_false_ignores_subdirs`
- [ ] `cargo test -p arawn-feeds` green

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

**2026-05-25 — Done.** Replaced T-A's `unimplemented!()` with the scan-and-diff core in `clients/filesystem.rs`, factored into pure helpers for testability:
- `parse_cursor()` — null/malformed cursor → empty map (drives first-run auto-index).
- `build_matcher()` — compiles include/exclude globs into `GlobSet`s.
- `scan()` — `WalkDir` over root (`max_depth(1)` when `recursive=false`), files only, matched against path **relative to root**; `exclude` always wins. Fingerprint = `(mtime secs, size)`.
- `diff()` — `created`/`modified`/`deleted`; unchanged emits nothing.
- `signal()` — documented shape `{source, path, rel_path, event, ts, size_bytes, mtime}`; `size_bytes`/`mtime` null on delete.
- `run()` appends one JSONL line per signal to `<feed_dir>/signals.jsonl` (append-only, mirrors stub's `log.jsonl`), persists the new fingerprint map as cursor, status `ok`/`no-changes`.
- Added `walkdir = "2"` to workspace + arawn-feeds.

20/20 module tests green (`cargo test -p arawn-feeds --lib filesystem`), incl. all six required cases. The `modified` test grows the file so the fingerprint differs regardless of mtime-second granularity (same-second in-place edit preserving size is the known undetected case, accepted for polling). `signals.jsonl` is what T-C's projection reader consumes.