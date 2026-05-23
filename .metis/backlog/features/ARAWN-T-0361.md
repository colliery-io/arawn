---
id: copy-slash-command-copy-last
level: task
title: "/copy slash command — copy last assistant response to clipboard"
short_code: "ARAWN-T-0361"
created_at: 2026-05-19T16:00:00+00:00
updated_at: 2026-05-19T15:10:03.333896+00:00
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

# /copy slash command — copy last assistant response to clipboard

## Backlog Item Details

### Type
- [x] Feature

### Priority
- [x] P3 — Low (nice ergonomic; not blocking)

## Objective

Add a `/copy` slash command that copies the most recent
assistant message body to the system clipboard. Useful for
"paste that response into a doc / Slack / wherever."

Filed from the I-0011 review (2026-05-19) — most of that
initiative's features became defunct after the I-0035 persona
shift, but `/copy` is universally useful regardless of framing.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `/copy` registered in the slash command registry.
- [ ] On invocation, walks `app.messages` backwards to find the
      most recent `ChatRole::Assistant` entry, sends its
      `content` to the system clipboard via a small clipboard
      crate (e.g. `arboard` — pick whatever has the lightest
      dep tree at the time).
- [ ] Posts a `/copy`-confirmation toast: `Copied last response
      (N chars) to clipboard.` Use the T-0359 toast surface.
- [ ] Empty transcript / no assistant turn → toast: `No
      assistant response yet to copy.`
- [ ] Clipboard error → error-level toast with the message.
- [ ] `angreal test unit` green.

## Implementation Notes

- The slash command itself is small (~30 LOC). Most of the
  work is choosing and adding a clipboard dep.
- `arboard` is a reasonable default — works across macOS,
  Linux (X11/Wayland), Windows.
- The handler runs synchronously from the slash command
  dispatch path; no async needed.

## Status Updates

### 2026-05-19 — /copy shipped (OSC 52, zero-dep)

- **Slash command** `/copy` registered in
  `arawn-tui::command::CommandRegistry::register_builtins`.
  Dispatch through new `CommandResult::CopyLastResponse`,
  handled locally in `App::handle_copy_last_response` (purely
  synchronous — no WS round-trip).
- **Behavior:**
  - Walks `app.messages` in reverse for the most recent
    `ChatRole::Assistant` entry.
  - Hands the body to `crate::toast::write_osc52_clipboard`
    which writes `ESC ] 52 ; c ; <base64> BEL` to stderr.
  - Posts an info-level toast: `"Copied last response (N
    chars) to clipboard."`
  - Empty / no-assistant case posts a warn-level toast:
    `"No assistant response yet to copy."`
- **Zero-dep clipboard path.** No `arboard` or other clipboard
  crate added — OSC 52 covers iTerm2, kitty, Alacritty,
  wezterm, recent xterm, tmux ≥ 3.3. Older terminals silently
  no-op; the toast still confirms intent.
- **Base64 encoder** inline in `toast.rs` (RFC 4648, full
  padding). 5 unit tests covering empty / 1-byte / 2-byte /
  3-byte / 6-byte inputs.
- **App-level tests (3 new):**
  - `copy_last_response_posts_toast_with_assistant_text`
  - `copy_last_response_warns_when_no_assistant_messages`
  - `copy_last_response_picks_most_recent_assistant_turn`
- `cargo test -p arawn-tui --lib` 217/0 (209 prior + 8 new).
  `angreal check workspace` green.