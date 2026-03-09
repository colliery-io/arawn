---
id: tui-cli-view-operational-logs-and
level: task
title: "TUI/CLI: View operational logs and chat history including tool calls"
short_code: "ARAWN-T-0274"
created_at: 2026-03-06T13:50:02.107768+00:00
updated_at: 2026-03-06T14:56:50.555091+00:00
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

# TUI/CLI: View operational logs and chat history including tool calls

## Objective

Provide visibility into operational logs and full chat history (including tool calls and their results) both in the TUI and via CLI commands.

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P1 - High (important for user experience)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] TUI has a view/panel to browse operational logs (server logs, errors, warnings)
- [ ] TUI chat view shows tool calls and their results inline (not just user/assistant text)
- [ ] CLI command to view session chat history with tool calls (e.g. `arawn session show <id>`)
- [ ] CLI command to tail/view operational logs (e.g. `arawn logs`)

## Status Updates

### Implementation Complete

**AC #1: TUI logs panel** — Already existed (`arawn-tui/src/logs.rs` + `ui/logs.rs`). Custom `TuiLogLayer` tracing subscriber captures logs into ring buffer, rendered with color-coded levels.

**AC #2: TUI chat shows tool calls inline** — Already existed (`arawn-tui/src/ui/chat.rs`). Tool executions rendered with name, args preview, status indicator (◐/✓/✗), and duration.

**AC #3: CLI `arawn session` command** — NEW
- `arawn session list` — lists all sessions with ID, timestamp, message count
- `arawn session show <id>` — displays full conversation history with tool calls
  - User messages with `> ` prefix
  - Assistant text
  - Tool calls showing name + truncated args
  - Tool results with ✓/✗ status + output preview
  - `--json` flag for machine-readable output

**AC #4: CLI `arawn logs` command** — NEW
- `arawn logs` — shows last 25 lines of latest log file
- `arawn logs -n 50` — custom line count
- `arawn logs -f` — follow mode (tail -f)
- `arawn logs --file <name>` — read specific log file
- Strips ANSI escape codes for clean output

**Server-side changes:**
- Extended `MessageInfo` to include optional `metadata` field
- `get_session_messages_handler` now emits `tool_use` and `tool_result` messages (previously only user/assistant)

**Files created:** `commands/session.rs`, `commands/logs.rs`
**Files modified:** `commands/mod.rs`, `main.rs`, `client/mod.rs`, `arawn-server/src/routes/sessions.rs`, `arawn-client/src/types.rs`

All tests pass. Workspace compiles clean.

### Server-Side Logs API — NEW

User requested a server-side logs API so TUI/remote clients can fetch server logs without filesystem access.

**New endpoint: `GET /api/v1/logs`**
- Query params: `lines` (default 50, max 1000), `file` (log file name)
- Returns `LogsResponse` with file name, count, and log entries
- Reads from `~/.config/arawn/logs/` (respects `ARAWN_CONFIG_DIR` env var)

**New endpoint: `GET /api/v1/logs/files`**
- Lists available log files with name and size
- Returns `LogFilesResponse`

**CLI `--remote` flag:**
- `arawn logs --remote` — fetches logs from running server via API
- `arawn logs --remote --list-files` — lists server log files
- `arawn logs --remote -n 100` — custom line count from server
- Supports `--json` output

**Files created:** `arawn-server/src/routes/logs.rs`
**Files modified:** `arawn-server/src/routes/mod.rs`, `arawn-server/src/lib.rs`, `arawn-server/Cargo.toml` (added `dirs`), `commands/logs.rs` (added `--remote`/`--list-files`), `client/mod.rs` (added `get_logs`/`list_log_files` methods + types)