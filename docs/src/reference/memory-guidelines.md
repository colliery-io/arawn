# Memory Guidelines

Operational rules for agent memory usage. These guidelines are injected into the
agent's system prompt context.

## Storage Rules

### Store

| Category | Examples |
|----------|---------|
| User-stated facts | Explicit preferences, biographical details |
| Research discoveries | Key findings from web search or code analysis |
| Corrections | Updates to previously stored incorrect information |
| Project context | Tech stack, conventions, deployment targets |
| Decisions | Key decisions and their rationale |

### Do Not Store

| Category | Reason |
|----------|--------|
| Temporary or session-specific information | Use notes instead |
| Sensitive data (passwords, API keys, tokens) | Security risk |
| Uncertain or unverified information | Pollutes memory with noise |
| Trivial details | Low recall value |
| Information already in files | Avoid duplication |

### Hygiene Rules

| Rule | Description |
|------|-------------|
| Supersede contradictions | Update contradictory information rather than storing duplicates |
| Cite sources | Include source attribution when storing from external content |
| Be specific | Prefer specific facts over vague observations |
| Preserve voice | Store preferences in the user's own words when possible |

---

## Memory Tool Usage

### memory_search

| Trigger | Action |
|---------|--------|
| User asks about past discussions or decisions | Search before answering |
| Need project-specific context | Query with relevant keywords |
| Looking for stored preferences | Search and cross-reference with current context |
| Building on previous research | Recall prior findings |

Guidelines: Query memory before answering factual questions about the user or
project. Use specific keywords. Acknowledge when memory informs the response.

### note

| Aspect | Rule |
|--------|------|
| Scope | Session-scoped (ephemeral, not persisted across sessions) |
| Titles | Use clear, searchable titles |
| Duplicates | Update existing notes rather than creating duplicates |
| Use for | Tracking session progress, intermediate results, current task reference |

**Note vs. Memory:** Notes are session-scoped and ephemeral. Memory is persistent
across sessions. Use memory for facts worth retaining long-term.

### think

| Trigger | Example |
|---------|---------|
| Complex or multi-step questions | Breaking down a debugging problem |
| Planning tool call sequences | Deciding which files to read first |
| Weighing trade-offs | Comparing implementation approaches |
| Correcting understanding | Revising assumptions after new information |
| Observing patterns | Noting user preferences for future reference |

Format:

```
think: "The user is working on a Rust project with async/await.
       They prefer explicit error handling over unwrap().
       The codebase uses tokio for the async runtime."
```
