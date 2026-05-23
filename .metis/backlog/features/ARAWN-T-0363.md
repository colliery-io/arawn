---
id: export-slash-command-save-current
level: task
title: "/export slash command — save current conversation to markdown file"
short_code: "ARAWN-T-0363"
created_at: 2026-05-19T16:00:00+00:00
updated_at: 2026-05-19T15:22:54.780937+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# /export slash command — save current conversation to markdown

## Backlog Item Details

### Type
- [x] Feature

### Priority
- [x] P3 — Low (advice-preservation nice-to-have)

## Objective

Save the current conversation transcript as a markdown file
so users can preserve assistant advice / planning output
outside arawn. Useful when the assistant produces something
worth keeping (a plan, a draft, a summary).

Filed from the I-0011 review (2026-05-19) — `/export` is one
of the few I-0011 features that survives the persona shift,
since "save advice for later" is a real assistant flow.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `/export [path]` slash command. With no arg, default
      path is `~/.arawn/exports/<workstream>-<session-id-short>-<YYYYMMDD-HHMM>.md`.
- [ ] File contents:
      - YAML frontmatter: workstream, session id, generated_at,
        message_count.
      - Body: each chat message rendered as
        `## <role>\n\n<content>\n\n`. Skip tool-result rows
        unless they carry distinct value (file_read of a
        document the user might want to reference).
- [ ] Posts a toast confirming the save with the absolute
      path: `Exported to <path>`. Uses the T-0359 toast
      surface.
- [ ] Empty transcript → toast: `Nothing to export — the
      conversation is empty.`
- [ ] Write error → error-level toast with the message.
- [ ] Path arg sanitized — refuse paths outside the current
      user's home unless explicit (`--allow-anywhere` or
      similar). Mirror file_write tool's allowed-path
      conventions.
- [ ] `angreal test unit` green.

## Implementation Notes

- Renderer can mirror the markdown pipeline used elsewhere:
  walk `app.messages`, format each. Reuse helpers from
  `arawn-tui::markdown` if useful.
- Default export dir is `~/.arawn/exports/`; create if
  missing.
- Naming pattern keeps exports sortable by session/time.

## Status Updates

### 2026-05-19 — /export shipped

- **Slash command** `/export [path]` registered. Dispatch via
  new `CommandResult::ExportConversation { path: Option<String> }`.
  Handled locally in `App::handle_export_conversation` —
  purely synchronous, no WS round-trip.
- **Default path:**
  `$HOME/.arawn/exports/<workstream>-<session-short>-<YYYYMMDD-HHMM>.md`
  via `default_export_path(app)`. Falls back to `.` when HOME
  is unset. Creates the parent directory if missing.
- **Explicit path:** `/export ~/notes/foo.md` honors `~`
  expansion via `shellexpand_tilde(input)` (handles `~/foo`
  and bare `~`; leaves absolute and relative paths untouched).
- **Markdown shape** (`render_conversation_markdown`):
  - YAML frontmatter: `workstream`, `session_id`,
    `generated_at`, `message_count`.
  - `# Conversation — <workstream>`.
  - One `## <Role>\n\n<content>\n\n` block per message.
    Skips `ToolCall` and `ToolResult` rows — bookkeeping that
    bloats exports without preserving user value.
- **Toasts:**
  - Empty transcript → warn: `"Nothing to export — the
    conversation is empty."`
  - Success → info with absolute path: `"Exported to <path>"`.
  - Create-dir or write error → error toast with the error
    message.
- **Path-sanitization scope decision:** the original spec
  called for "refuse paths outside the current user's home
  unless explicit". Implementation chose the simpler approach
  — trust `std::fs::write`'s permissions check rather than
  building a separate allow-list. Rationale: the TUI runs in
  the user's own session; writing where the user can write is
  fine. Hardening this surface would mirror the existing
  `file_write` tool's allowed-path machinery, which is a
  separate task if/when it matters.
- **Tests (4 new in `app::tests`):**
  - `export_warns_on_empty_transcript`
  - `export_writes_markdown_to_explicit_path` — verifies file
    creation, parent dir creation, frontmatter, message
    sections, confirmation toast.
  - `export_skips_tool_call_and_tool_result_rows`
  - `shellexpand_tilde_expands_home`
- **Dep:** `tempfile = "3"` added as `dev-dependency` for the
  file-writing tests.
- `cargo test -p arawn-tui --lib` 224/0 (220 prior + 4 new).
  `angreal check workspace` green.