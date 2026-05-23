---
id: cross-session-memory-extract
level: task
title: "Cross-session memory — extract, persist, and inject relevant memories"
short_code: "ARAWN-T-0038"
created_at: 2026-04-01T11:02:02.134822+00:00
updated_at: 2026-04-01T11:02:02.134822+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Cross-session memory — extract, persist, and inject relevant memories

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

Enable Arawn to learn across sessions. At the end of each session (or on compaction), extract key facts, preferences, and decisions into a memory store. At the start of new sessions, scan memories for relevance to the current workstream/query and inject the top-N into the system prompt. This is the graphqlite use case from the vision — memories form a knowledge graph that grows over time.

Claude Code's approach: file-based memories in `~/.claude/` with auto-extraction, relevance matching, and team sync. Arawn's approach: start with file-based (like `.arawn.md` but auto-generated), graduate to graphqlite when the knowledge graph initiative lands.

### Priority
- P3 — high value but large scope. Depends on context file support (T-0037) as a simpler foundation first.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `Memory` type: `id`, `content`, `source_session_id`, `workstream_id`, `created_at`, `tags`
- [ ] Memory extraction: at session end or compaction, call LLM with "extract key facts from this conversation" prompt
- [ ] Extracted memories stored as markdown files in `~/.arawn/memories/` (one file per memory, frontmatter for metadata)
- [ ] Memory scanning: on session start, read all memories for the current workstream
- [ ] Relevance ranking: score memories against the user's first message (simple keyword overlap or LLM-based)
- [ ] Top-N memories (configurable, default 5) injected into system prompt as `# Relevant Memories` section
- [ ] Memory persistence survives across sessions (file-based, no DB needed initially)
- [ ] User can manually add memories: `arawn remember "always use cargo test --workspace"`
- [ ] User can list/delete memories: `arawn memories list`, `arawn memories delete <id>`
- [ ] Test: memory extracted from session, loaded in new session with same workstream
- [ ] Test: memories from different workstreams are isolated

## Implementation Notes

- Start simple: markdown files in `~/.arawn/memories/<workstream_id>/`. Each file = one memory.
- Extraction prompt: "List 3-5 key facts, decisions, or preferences from this conversation that would be useful in future sessions. Be specific and concise."
- Relevance: v1 can be simple keyword match against first user message. v2 uses embedding similarity via graphqlite.
- This is the bridge to graphqlite — file-based memories are the interim store, graph becomes the long-term store.
- Depends on: T-0037 (context file support for the injection mechanism)

### Knowledge Graph Direction
The long-term vision is NOT flat file memories like Claude Code's MEMORY.md. Instead, build toward a knowledge graph that captures entities, relationships, and provenance:
- Memories are nodes with typed relationships (user_prefers, project_uses, decision_made, etc.)
- Retrieval is graph traversal, not keyword search — "what do I know about the auth system?" follows edges
- Entity deduplication — the same concept referenced across sessions collapses to one node
- Provenance tracking — each fact knows which session/conversation produced it
- Start with file-based extraction (v1), add graph storage via graphqlite/SQLite (v2), add embedding-based similarity for relevance ranking (v3)
- This differentiates arawn from Claude Code's flat memory approach — a KB that compounds over time

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