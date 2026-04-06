---
id: first-class-workstreams-tui
level: initiative
title: "First-class workstreams — TUI navigation, agent tools, and session binding"
short_code: "ARAWN-I-0018"
created_at: 2026-04-06T10:00:15.118843+00:00
updated_at: 2026-04-06T10:00:15.118843+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: first-class-workstreams-tui
---

# First-class workstreams — TUI navigation, agent tools, and session binding Initiative

## Context

Workstreams are arawn's core organizational concept — they provide isolated contexts for different areas of life (projects, finances, health, etc.) with separate sessions, sandboxed file access, and workstream-scoped memory/context. The infrastructure exists:

- **Store**: `create_workstream`, `list_workstreams`, `promote_session` (scratch → workstream)
- **Server**: `list_workstreams` and `create_workstream` JSON-RPC methods
- **TUI**: `workstreams` and `current_workstream` state, sidebar shows workstreams
- **Engine**: `Workstream` struct with name, root_dir, id; `ToolContext` scoped to workstream
- **JSONL**: Messages stored under `workstreams/<name>/` directories
- **Sandbox**: Shell tool scoped to workstream root directory

But none of this is **usable** today:

1. **TUI sidebar**: Shows workstreams but clicking doesn't switch — sessions always go to scratch
2. **No agent tools**: The agent can't create workstreams, switch between them, or promote sessions
3. **No session binding UX**: Creating a new session always lands in scratch, no way to start a session in a specific workstream
4. **No workstream awareness**: The agent doesn't know which workstream it's in or what workstreams exist
5. **No `/workstream` command**: No user command to create or switch workstreams from the TUI input

## Goals & Non-Goals

**Goals:**
- TUI sidebar: click a workstream to switch to it, show its sessions, create new sessions within it
- Agent tools: `WorkstreamCreate`, `WorkstreamList`, `WorkstreamSwitch` tools the agent can call
- TUI commands: `/workstream create <name>`, `/workstream switch <name>`, `/workstream list`
- Session promotion: `/promote <workstream>` command to move a scratch session into a workstream
- Workstream context in system prompt: agent knows which workstream it's in, can reference workstream-specific arawn.md
- Status bar shows current workstream name

**Non-Goals:**
- Workstream-level permissions or access control
- Cross-workstream queries (search across all workstreams)
- Workstream archiving/deletion (can add later)
- Workstream templates or presets

## Use Cases

### Use Case 1: Create and switch to a project workstream
- **Actor**: User via TUI
- **Scenario**: User types `/workstream create arawn-dev`. Workstream is created with its own directory. Sidebar updates to show it. A new session is created within the workstream. The system prompt now includes "Workstream: arawn-dev" and the workstream's arawn.md if it exists.
- **Expected Outcome**: User is working in an isolated context for arawn development.

### Use Case 2: Agent creates a workstream
- **Actor**: Agent during conversation
- **Scenario**: User says "I want to start tracking my home maintenance tasks." Agent calls `WorkstreamCreate` with name "home-maintenance". Agent acknowledges the creation and offers to switch to it.
- **Expected Outcome**: Workstream exists, agent is aware of it.

### Use Case 3: Promote a scratch session
- **Actor**: User via TUI
- **Scenario**: User has been working in a scratch session on a topic and realizes it belongs in a workstream. Types `/promote finances`. The session and its messages move from scratch to the finances workstream. Sandbox files are migrated.
- **Expected Outcome**: Session now lives in the workstream with all history preserved.

### Use Case 4: Switch between workstreams
- **Actor**: User via TUI sidebar
- **Scenario**: User clicks "finances" in the sidebar. The TUI switches to show only that workstream's sessions. A new session is created or the most recent one is resumed. The agent's context updates to the workstream's root directory and arawn.md.
- **Expected Outcome**: Seamless context switch between workstreams.

## Detailed Design

### What exists today

```
Server (ws_server.rs):
  - list_workstreams → returns Vec<WorkstreamInfo>
  - create_workstream(name, root_dir) → creates in store + filesystem
  - create_session(workstream_id) → already supports binding to workstream
  - send_message(session_id, content) → engine runs in session's workstream context

Store (store.rs):
  - create_workstream, list_workstreams, find_workstream_by_name
  - promote_session(session_id, workstream_id) → moves JSONL + updates SQLite
  - sandbox_for(workstream_dir, session_id) → resolves sandbox path

TUI (app.rs):
  - workstreams: Vec<WorkstreamInfo> (loaded on startup)
  - current_workstream: Option<WorkstreamInfo>
  - Sidebar renders workstream names (clickable but no action)
```

### What needs to be built

**Phase 1: TUI workstream navigation**
- Clicking a workstream in sidebar: sets `current_workstream`, loads its sessions, creates a new session if none exist
- New session creation uses `create_session(Some(workstream_id))` instead of always `None` (scratch)
- Status bar shows current workstream name
- Sidebar groups sessions under their workstream

**Phase 2: TUI commands**
- `/workstream create <name>` → calls `create_workstream` RPC, switches to it
- `/workstream list` → shows all workstreams with session counts
- `/workstream switch <name>` → switches TUI context
- `/promote <workstream>` → calls promote endpoint, moves current session

**Phase 3: Agent tools**
- `WorkstreamCreate` tool — agent can create workstreams
- `WorkstreamList` tool — agent can list available workstreams
- `WorkstreamSwitch` tool — agent can switch the active workstream (tricky — needs to update TUI state)
- Register in both serve and CLI modes

**Phase 4: Workstream-scoped context**
- System prompt includes workstream name and root directory (already partially done via `PromptContext`)
- Workstream's `arawn.md` injected into prompt (already wired but depends on correct working dir)
- Memory system scoped to workstream (already has workstream-level KB)
- Verify sandbox enforces workstream boundaries

### Server-side changes needed

New JSON-RPC methods:
- `switch_workstream(workstream_id)` → changes the connection's active workstream, loads sessions
- `promote_session(session_id, workstream_id)` → wraps `store.promote_session`
- `list_workstream_sessions(workstream_id)` → already exists as `list_sessions`

### TUI state changes

```rust
// App state additions
pub current_workstream: Option<WorkstreamInfo>,
pub workstream_sessions: HashMap<Uuid, Vec<SessionInfo>>, // cached per-workstream

// On workstream click:
fn switch_workstream(&mut self, ws: WorkstreamInfo) {
    self.current_workstream = Some(ws.clone());
    self.sessions = self.workstream_sessions.get(&ws.id).cloned().unwrap_or_default();
    // Create new session if empty
}
```

## Alternatives Considered

- **Flat session list (no workstreams)**: Simpler but doesn't scale — users with 50+ sessions need organization
- **Tags instead of workstreams**: More flexible but less structured — harder to scope sandbox/context
- **Auto-detect workstream from topic**: Interesting but unreliable — explicit assignment is clearer

## Implementation Plan

**Phase 1**: TUI workstream navigation — sidebar click, session grouping, status bar
**Phase 2**: TUI commands — `/workstream create|list|switch`, `/promote`
**Phase 3**: Agent tools — WorkstreamCreate, WorkstreamList
**Phase 4**: Verify workstream-scoped context — prompt, memory, sandbox all respect active workstream