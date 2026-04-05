---
id: lsp-tool-code-intelligence-go-to
level: task
title: "LSP Tool — code intelligence (go-to-definition, find references, hover, symbols)"
short_code: "ARAWN-T-0051"
created_at: 2026-04-03T01:01:31.328534+00:00
updated_at: 2026-04-03T01:01:31.328534+00:00
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

# LSP Tool — code intelligence (go-to-definition, find references, hover, symbols)

## Objective

Add an LSP tool that provides code intelligence features by connecting to Language Server Protocol servers. This gives the agent the ability to navigate code structurally rather than relying solely on text search.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [ ] P1 - High (important for user experience)

### Business Justification
- **User Value**: Enables precise code navigation — jump to definitions, find all references, get type info, list symbols. Far more accurate than grep for understanding code structure.
- **Effort Estimate**: XL — requires LSP client infrastructure, server lifecycle management, and configuration per language.

## Acceptance Criteria

## Acceptance Criteria

- [ ] LSP tool supports operations: goToDefinition, findReferences, hover, documentSymbol, workspaceSymbol, goToImplementation
- [ ] Operations take filePath, line, character params (1-based)
- [ ] LSP servers are configurable per language/file type
- [ ] Server lifecycle managed (start on first use, shutdown on session end)
- [ ] Graceful error when no LSP server configured for file type
- [ ] Call hierarchy support (prepareCallHierarchy, incomingCalls, outgoingCalls)

## Implementation Notes

### Technical Approach
- Use `tower-lsp` or raw LSP JSON-RPC over stdio
- Server config in `arawn.toml` (path to binary, args, file patterns)
- Lazy server startup — only spawn when the tool is first called for a file type
- Cache server connections per language for the session lifetime
- Reference: Claude Code's `LSPTool/` implementation

### Dependencies
- None on existing tools, but benefits from FileReadTool (for knowing which files are open)

### Risk Considerations
- LSP servers can be slow to start (rust-analyzer: 10-30s on large projects)
- Memory usage — some servers hold entire project in memory
- Server crashes need graceful recovery