---
id: session-memory-extraction-auto
level: task
title: "Session memory extraction — auto-extract key facts on compaction for future sessions"
short_code: "ARAWN-T-0063"
created_at: 2026-04-03T02:03:50.874725+00:00
updated_at: 2026-04-03T02:03:50.874725+00:00
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

# Session memory extraction — auto-extract key facts on compaction for future sessions

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

When compaction fires, run a secondary extraction pass that pulls key facts, decisions, and preferences from the conversation being summarized. Store these as memory entries that feed into the cross-session memory system (T-0038). This is the bridge between compaction and persistent memory — compaction loses detail, but the extraction preserves the important bits.

### Type: Feature | Priority: P2 | Effort: M

## Acceptance Criteria

## Acceptance Criteria

- [ ] On compaction, after summarizing, run an extraction prompt: "List key facts, decisions, preferences from this conversation"
- [ ] Extracted items stored as memory entries in `~/.arawn/memories/<workstream>/`
- [ ] Each memory has: content, source_session_id, created_at, tags
- [ ] Extraction uses the same LLM as compaction (no extra config)
- [ ] Extraction is best-effort — failure doesn't block compaction
- [ ] Memories deduplicated against existing entries (don't re-extract the same fact)

## Implementation Notes

- Hook into `Compactor::compact()` — after summary is generated, run extraction prompt on the pre-compaction messages
- Extraction prompt: "From this conversation, extract 3-5 key facts that would be useful in future sessions. Output as JSON array of {content, tags}."
- Write to `~/.arawn/memories/<workstream_id>/<uuid>.md` with YAML frontmatter
- Dedup: compare new extractions against existing memories using simple string similarity
- Depends on: T-0038 (memory system provides the storage format and injection mechanism)
- Reference: Claude Code's `sessionMemoryCompact.ts`