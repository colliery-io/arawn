---
id: cli-assistant-framing-empty-chat
level: task
title: "CLI assistant framing + empty-chat welcome message"
short_code: "ARAWN-T-0331"
created_at: 2026-05-18T20:18:06.800722+00:00
updated_at: 2026-05-19T00:55:30.807329+00:00
parent: ARAWN-I-0035
blocked_by: [ARAWN-T-0330]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0035
---

# CLI assistant framing + empty-chat welcome message

## Parent Initiative

[[ARAWN-I-0035]]

## Objective

Close the user-visible identity gap that `arawn --help` and the
empty-chat state still call arawn an "LLM-powered coding
assistant" and present a blank input box. After [[ARAWN-T-0330]]
fixes the model's prompt, this task fixes what the *user* sees
before they type their first message.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `crates/arawn/src/main.rs:26` — the clap `about = "..."`
      string is rewritten for the assistant framing. Suggested:
      `"Personal agentic assistant — watch, check, summarize,
      and nudge across your tools."` Final wording up to author
      taste, but must drop "coding".
- [ ] `arawn --help` and `arawn -h` reflect the new framing.
      `arawn --version` is untouched.
- [ ] README.md top-line description and any `crate` description
      attributes (`description = ...` in `Cargo.toml`) updated
      to match. Search for "coding assistant" repo-wide; rewrite
      every user-facing instance. Internal comments / test
      strings that say "coding" are out of scope.
- [ ] TUI empty-chat state shows a welcome system message before
      any user input has been entered. Suggested text:
      `"Welcome to arawn. Type / for commands, /connect to wire
      up Gmail/Calendar/Slack, or just chat."` Renders only when
      the conversation has zero turns; suppresses itself the
      moment the first user/assistant turn appears.
- [ ] Welcome text uses neutral framing (no decorative palette /
      panel chrome) so it doesn't conflict with the I-0036 visual
      coherence pass landing later.
- [ ] Unit test for the empty-state predicate (welcome shows iff
      transcript is empty, hides on first turn). If the welcome
      lives at the renderer layer with no easy seam, a
      golden-string snapshot of the empty-state render is
      acceptable.
- [ ] `angreal test unit` green. `angreal check workspace` green.
- [ ] Manual: `cargo run -- --help` shows the new framing;
      launching the TUI on a fresh session shows the welcome;
      typing a message hides it.

## Implementation Notes

### Technical Approach

1. **CLI metadata.** Edit `crates/arawn/src/main.rs:26` and any
   `Cargo.toml` `description` fields. README top of file.
2. **Welcome message.** Probably lives in
   `crates/arawn-tui/src/app.rs` or the transcript-render path.
   Cleanest seam: a derived `should_show_welcome(&self) -> bool`
   on the transcript state (`turns.is_empty()`); render emits a
   styled system-message row when true. Avoid storing the
   welcome as a real turn — keep transcript state authoritative.
3. **Repo sweep.** `rg -i "coding assistant"` from repo root;
   rewrite each user-facing hit. Skip test fixtures and comments
   that document historical state.

### Dependencies

- Soft-depends on [[ARAWN-T-0330]] (no code overlap, but it's
  weird to ship the user-facing framing while the engine prompt
  still says "you are an agent that BUILDS things"). Mark
  `blocked_by: [ARAWN-T-0330]` to enforce order.

### Risk Considerations

- Welcome message that's too long is its own kind of "coding
  REPL chrome" — keep it to one line.
- README rewrites can churn unrelated marketing copy; scope to
  the headline and "what is arawn" paragraph only.

## Status Updates

### 2026-05-18 — CLI framing + welcome shipped

- `crates/arawn/src/main.rs:44` clap `about` rewritten to
  `"Personal agentic assistant — watch, check, summarize, and
  nudge across your tools."` Verified via
  `cargo run --bin arawn -- --help`.
- README + `Cargo.toml` repo sweep: README already said
  "personal agentic assistant"; no Cargo `description` fields
  needed updating. `rg -i "coding assistant"` only hits archived
  Metis docs and a `review/` snapshot — both historical and
  out of scope.
- TUI idle hero (`crates/arawn-tui/src/render.rs::render_idle_hero`)
  now renders a two-line welcome between the `arawn` box and the
  existing hint lines:
    "Welcome — your personal agentic assistant."
    "I watch, check, summarize, and nudge across your tools."
  Hint lines retained verbatim ("Type / for commands · Tab to
  toggle sidebar" / "/connect <service> · ↑ recall"). Hero box
  widened from 44 → 56 cells to fit the longer text.
- Welcome uses the existing `theme::SUBTEXT0` palette (already
  used for "arawn" lettering) — no new colors introduced.
  Stays neutral so it won't conflict with whatever I-0036
  (TUI visual coherence pass) eventually produces.
- Existing predicate `app.messages.is_empty() && !app.is_generating
  && app.streaming_text.is_empty()` already implements the
  "renders only when transcript is empty" requirement. No new
  state needed.
- 8 ratatui snapshots accepted: `snapshot_empty_app`,
  `snapshot_focus_main`, `snapshot_focus_sidebar`,
  `snapshot_idle_hero`, `snapshot_input_placeholder`,
  `snapshot_sidebar_with_workstreams`,
  `styled_snapshot_focus_borders`,
  `styled_snapshot_sidebar_focused`. First 7 capture the
  rendered hero; the 8th shifted by 1 cell because the wider
  hero box changes neighboring chrome alignment.
- `angreal test unit` green. The existing snapshot regression is
  the empty-state predicate test (the hero only renders when the
  transcript is empty).