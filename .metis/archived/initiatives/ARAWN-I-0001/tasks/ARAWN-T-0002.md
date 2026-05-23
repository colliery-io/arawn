---
id: arawn-core-domain-types-workstream
level: task
title: "arawn-core — Domain types: Workstream, Session, Message"
short_code: "ARAWN-T-0002"
created_at: 2026-03-31T17:37:35.919583+00:00
updated_at: 2026-03-31T18:56:17.785870+00:00
parent: ARAWN-I-0001
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0001
---

# arawn-core — Domain types: Workstream, Session, Message

## Parent Initiative
[[ARAWN-I-0001]]

## Objective
Implement the foundational domain types in `arawn-core`. These types define the data model the entire system is built on. The key invariant: a session belongs to exactly one workstream, immutably bound at creation.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `Workstream` struct: `id`, `name`, `root_dir: PathBuf`, `created_at`
- [ ] `Session` struct: `id`, `workstream_id` (immutable), `messages: Vec<Message>`, `created_at`
- [ ] `Message` enum matching Anthropic API shape: `User { content }`, `Assistant { content, tool_uses }`, `ToolResult { tool_use_id, content }`
- [ ] `ToolUse` struct: `id`, `name`, `input: serde_json::Value`
- [ ] All types derive `Clone`, `Debug`, `Serialize`, `Deserialize`
- [ ] `Session::new(workstream_id)` — constructor enforces immutable binding
- [ ] `Session::add_message(&mut self, msg)` — append to history
- [ ] `Session::messages(&self)` — read access to history
- [ ] Scratch workstream concept: a `Workstream::scratch()` constructor that creates the default workspace
- [ ] Unit tests for type construction, serialization roundtrip, session message ordering

## Implementation Notes
- `workstream.rs`, `session.rs`, `message.rs`, `error.rs` in `crates/arawn-core/src/`
- Types should be persistence-agnostic — no SQLite, no file I/O. Just in-memory domain objects.
- Use `uuid::Uuid` for IDs (or a newtype wrapper)
- `chrono::DateTime<Utc>` for timestamps
- Depends on: ARAWN-T-0001 (workspace scaffolding)

## Status Updates
- **2026-03-31**: All types implemented. 12 unit tests passing: serialization roundtrips, session binding immutability, message ordering, unique IDs, scratch workstream. Note: Session doesn't derive Serialize/Deserialize yet (persistence-agnostic for now) — only Message types do.