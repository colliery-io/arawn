# Navigate the TUI

This guide covers launching, navigating, and using every panel and keyboard shortcut in the Arawn terminal UI.

## Launch the TUI

Start with the default workstream:

```bash
arawn tui
```

Start directly in a specific workstream:

```bash
arawn tui -w "Backend API Redesign"
```

## Understand the layout

The TUI is divided into several panels:

```
┌──────────┬──────────────────────────────────────────┐
│ Sidebar  │  Chat Area                               │
│          │                                          │
│ Work-    │  Messages scroll here                    │
│ streams  │                                          │
│          │                                          │
│ Sessions ├──────────────────────────────────────────┤
│          │  Tool Pane (collapsible)                 │
│          │  Shows tool executions + output           │
│          ├──────────────────────────────────────────┤
│          │  Logs Panel (collapsible)                │
│          │  Debug/info log output                    │
├──────────┴──────────────────────────────────────────┤
│  Input Box                                          │
│  Type your message here...                          │
└─────────────────────────────────────────────────────┘
```

- **Sidebar** -- Lists workstreams (top) and sessions for the active workstream (bottom).
- **Chat area** -- Displays the conversation: user messages, assistant responses, and tool call summaries.
- **Tool pane** -- Shows details of each tool execution including status, timing, and truncated output.
- **Logs panel** -- Streams internal log messages (debug, info, warnings).
- **Input box** -- Where you type messages and slash commands.

## Keyboard shortcuts

### Sending messages

| Key | Action |
|-----|--------|
| Enter | Send the current message |
| Shift+Enter | Insert a newline (multi-line input) |
| Esc | Clear the input box, or cancel the current mode |

### Text editing

| Key | Action |
|-----|--------|
| Left / Right | Move cursor one character |
| Home | Jump to the start of the current line |
| End | Jump to the end of the current line |
| Backspace | Delete the character before the cursor |
| Delete | Delete the character at the cursor |

### History navigation

| Key | Context | Action |
|-----|---------|--------|
| Up | Input is empty | Scroll chat up by 1 line |
| Up | Cursor on first line | Browse to previous history entry |
| Up | Multi-line, cursor not on first line | Move cursor up one line |
| Down | Input is empty | Scroll chat down by 1 line |
| Down | Cursor on last line | Browse to next history entry (or restore draft) |
| Down | Multi-line, cursor not on last line | Move cursor down one line |

When browsing history, your current draft is saved. Navigate forward past the newest history entry to restore it. Typing any character exits history mode and commits the displayed text.

### Scrolling

| Key | Action |
|-----|--------|
| PageUp | Scroll chat up by 10 lines |
| PageDown | Scroll chat down by 10 lines |
| Ctrl+Home | Scroll to the top of the chat; disable auto-scroll |
| Ctrl+End | Scroll to the bottom of the chat; re-enable auto-scroll |

Auto-scroll keeps the view pinned to the bottom as new messages arrive. Scrolling up disables it so you can read history without being pulled back down. Ctrl+End or sending a new message re-enables it.

### Mouse

Scroll the mouse wheel over any panel to scroll its content:

- **Chat area** -- scroll conversation history
- **Tool pane** -- scroll tool execution output
- **Logs panel** -- scroll log entries
- **Sidebar** -- scroll the workstream or session list

## Use slash commands

Type `/` at the start of the input box to open the command popup. A filtered list of available commands appears above the input.

| Key | Action |
|-----|--------|
| `/` (at start of input) | Open the command popup |
| Up / Down | Navigate the command list |
| Tab or Enter | Complete the selected command |
| Esc | Close the popup |
| Continue typing | Filter the command list |
| Backspace | Update the filter (or close popup if only `/` remains) |

After completing a command, the input box shows `/<command> ` with a trailing space ready for arguments.

Type `/help` to see the full list of available commands.

## Manage sessions

### The sessions overlay

The sidebar displays sessions for the currently active workstream. Each session entry shows its creation time and status.

| Action | How |
|--------|-----|
| Switch sessions | Click a session in the sidebar, or navigate with arrow keys |
| Create a new session | Select "New Session" at the top of the session list, or press Ctrl+N |
| Filter sessions | Start typing while the session list is focused |

### Read-only mode

If another client (another TUI instance, the REST API, or a WebSocket connection) owns the active session, the TUI enters read-only mode. You can view the conversation but cannot send messages. Switch to a different session or create a new one to regain input.

## Work with the sidebar

The sidebar is split into two sections:

1. **Workstream list** -- All named workstreams plus the scratch workstream. Click or navigate to switch workstreams.
2. **Session list** -- Sessions belonging to the selected workstream. The most recent session is at the top.

### Create a new workstream

1. Select the "New Workstream" entry in the sidebar
2. The input box switches to workstream-naming mode (the status bar shows the prompt)
3. Type a name (up to 100 characters) and press Enter
4. The new workstream is created and becomes active

### Rename a workstream

Use the `/rename` slash command or the context menu in the sidebar to rename the current workstream.

## Monitor tool execution

The tool pane shows each tool call the agent makes during a response:

- **Tool name** -- Which tool was invoked (e.g., `shell`, `file_read`, `mcp__sqlite__query`)
- **Status** -- Running, completed, or failed
- **Timing** -- How long the execution took
- **Output** -- Truncated tool output (scroll to see more)

MCP tools are namespaced with a `mcp__<server>__<tool>` prefix so you can tell which server handled the call.

## Use the command palette

The command palette provides quick access to actions beyond slash commands. It integrates with the TUI's action system, giving you a searchable list of all available operations including workstream management, session control, and display toggles.

Open the palette from a slash command or keyboard shortcut, then type to filter and press Enter to execute.

## Handle focus and panels

The TUI tracks which panel has focus. Focus determines where keyboard input is directed:

- **Input box** -- Default focus. All typing goes into the message input.
- **Sidebar** -- When focused, arrow keys navigate the workstream and session lists.
- **Sessions overlay** -- A modal overlay for switching sessions. Arrow keys navigate, typing filters, and Esc closes it.

Clicking on a panel with the mouse switches focus to it. The focused panel is visually highlighted.

### Toggle panels

The tool pane and logs panel can be toggled on or off to give more room to the chat area. Use the appropriate slash command or palette action to show or hide them.

## Common workflows

### Start a new project

1. Launch the TUI: `arawn tui`
2. In the sidebar, select "New Workstream"
3. Type a descriptive name (e.g., "API Migration") and press Enter
4. A new session is created automatically -- start chatting

### Resume previous work

1. Launch the TUI (or use `arawn tui -w "API Migration"`)
2. The sidebar shows existing sessions for that workstream
3. Click the session you want to resume, or create a new one with Ctrl+N
4. The agent loads the full workstream conversation history

### Review tool calls

1. Keep the tool pane visible while the agent works
2. Each tool call appears with its name, status indicator, and duration
3. Scroll the tool pane to review output from earlier calls
4. Failed tool calls show error details in red

## Tips

- **Multi-line messages**: Use Shift+Enter to build up multi-line prompts before sending with Enter.
- **Quick scroll to bottom**: Press Ctrl+End after reading history to snap back to the latest messages and re-enable auto-scroll.
- **Interrupt streaming**: Press Esc while the agent is streaming a response to cancel it.
- **Persistent history**: Your input history (up to 100 entries) is preserved across sessions, so you can reuse previous messages.
- **Workstream context**: Switching workstreams in the sidebar loads that workstream's full conversation history and file context into the agent's view.
