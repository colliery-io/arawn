---
id: messagestore-jsonl-file-reader
level: task
title: "MessageStore — JSONL file reader/writer"
short_code: "ARAWN-T-0011"
created_at: 2026-03-31T22:49:35.403704+00:00
updated_at: 2026-03-31T23:11:54.348928+00:00
parent: ARAWN-I-0002
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0002
---

# MessageStore — JSONL file reader/writer

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0002]]

## Objective

Implement `MessageStore` that persists conversation messages as JSONL files on disk. One file per session, one JSON line per message, organized under the workstream's directory.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `MessageStore` trait: `append(session_id, workstream_id, msg)`, `load(session_id, workstream_id) → Vec<Message>`
- [ ] `JsonlMessageStore` implementation that reads/writes to `<data_dir>/workstreams/<ws_id>/sessions/<session_id>.jsonl`
- [ ] Scratch sessions stored at `<data_dir>/scratch/sessions/<session_id>.jsonl`
- [ ] `append` creates parent directories if needed, opens file in append mode, writes one JSON line
- [ ] `load` reads all lines, deserializes each as `Message`, returns in order
- [ ] `move_session(session_id, from_ws_id, to_ws_id)` — moves the JSONL file between directories (for promotion)
- [ ] Handles missing file gracefully: `load` on nonexistent session returns empty Vec
- [ ] Test: append 3 messages → load → verify order and content
- [ ] Test: append to same session twice → load → all messages present
- [ ] Test: load nonexistent → empty vec
- [ ] Test: move_session moves file correctly
- [ ] Test: JSONL is valid — each line independently parseable

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

## Implementation Notes
- `jsonl.rs` in `crates/arawn-storage/src/`
- Use `tokio::fs` for async file I/O, `serde_json` for serialization
- Append mode: `OpenOptions::new().create(true).append(true)` — no locking needed (single user)
- Each line is a complete `arawn_core::Message` serialized as JSON + `\n`
- `move_session` uses `tokio::fs::rename` (atomic on same filesystem)
- For scratch sessions, `workstream_id` is `None` — use `scratch/` directory path
- Tests use `tempdir` for isolation
- Depends on: ARAWN-T-0009 (crate exists), independent of T-0010 (SQLite stores)

## Status Updates
- **2026-03-31**: Complete. JsonlMessageStore with append/load/move_session. Async via tokio::fs. Scratch sessions in scratch/sessions/, workstream sessions in workstreams/<id>/sessions/. 7 new tests: roundtrip, accumulation, nonexistent load, scratch path, move/relocate, move nonexistent, JSONL validity. 26 total storage tests, clippy clean.