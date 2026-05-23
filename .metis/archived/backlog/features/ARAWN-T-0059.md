---
id: fileread-filewrite-pre-read
level: task
title: "FileRead/FileWrite pre-read enforcement — error if editing/writing a file not previously read"
short_code: "ARAWN-T-0059"
created_at: 2026-04-03T01:01:42.907701+00:00
updated_at: 2026-04-03T01:25:28.637597+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# FileRead/FileWrite pre-read enforcement — error if editing/writing a file not previously read

## Objective

Track which files the agent has read during a session, and enforce that FileEdit and FileWrite error if the target file exists but hasn't been read first. Prevents blind edits to files the agent hasn't seen, reducing hallucinated changes.

### Type: Feature | Priority: P2

- **User Value**: Prevents the agent from making blind edits to files it hasn't seen, reducing hallucinated or incorrect changes. Also enables "file unchanged since last read" optimization.
- **Effort Estimate**: S

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Session-scoped `HashSet<PathBuf>` tracks files read by FileReadTool
- [ ] FileEditTool errors with clear message if target file exists but hasn't been read
- [ ] FileWriteTool errors if target file exists but hasn't been read (new files are fine)
- [ ] "File unchanged" detection: if file content hash matches last read, return stub instead of full content
- [ ] Read tracking shared via ToolContext or similar session-scoped state

## Implementation Notes

- Add `read_files: Arc<RwLock<HashSet<PathBuf>>>` to ToolContext
- FileReadTool inserts path on successful read
- FileEditTool/FileWriteTool check the set before proceeding
- Optional: store content hash per file for unchanged detection (saves tokens on re-reads)
- Reference: Claude Code's `FILE_UNCHANGED_STUB` in FileReadTool and pre-read checks in FileEditTool/FileWriteTool