---
id: brief-slash-command-empty-chat
level: task
title: "/brief slash command + empty-chat auto-render"
short_code: "ARAWN-T-0354"
created_at: 2026-05-19T03:00:00+00:00
updated_at: 2026-05-19T12:34:08.769117+00:00
parent: ARAWN-I-0035
blocked_by: [ARAWN-T-0349]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0035
---

# /brief slash command + empty-chat auto-render

## Parent Initiative

[[ARAWN-I-0035]]

## Objective

Wire the `BriefView` composer from [[ARAWN-T-0349]] into two
user-facing paths:

1. **`/brief` slash command** — fetches today's daily tablet
   and this week's weekly tablet from the ceremony service,
   composes a `BriefView`, renders as a system message into
   the chat.
2. **Empty-chat auto-render** — when the transcript is empty
   AND at least one tablet exists for the current period, the
   idle hero in `arawn-tui::render::render_idle_hero` swaps the
   welcome message for the composed brief.

The T-0331 welcome line stays as the pre-onboarding state — it
renders only when no tablets exist yet. Transition is
data-driven; no flag.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `/brief` slash command registered in the TUI slash-command
      surface (wherever `/connect`, `/diag`, `/day` etc. live).
      Behavior:
      - Fetch today's daily tablet via the ceremony service
        (likely `daily_current` or equivalent service method).
        Fetch this week's weekly tablet via `weekly_current`.
        Either may be `None`.
      - Build `BriefView { daily, weekly }`.
      - Render via `render_brief`.
      - Append the rendered markdown as a system message in
        the transcript (same path as other slash-command
        outputs).
- [ ] `arawn-tui::render::render_idle_hero` updated:
      - Probe ceremony service for any current tablet (daily
        or weekly).
      - If at least one exists: render the composed brief in
        the empty-chat area instead of the welcome lines.
        Re-uses the existing markdown render path (inherits
        Catppuccin Mocha styling from I-0036).
      - If none exist: render the existing T-0331 welcome
        (`Welcome — your personal agentic assistant.` etc.).
- [ ] The tablet probe is cheap and synchronous-from-the-render's
      POV. Acceptable to:
      - Cache the most recently fetched tablets on `App` state
        and refresh on WebSocket ServerNotice events
        (`briefing_ready` if it exists, or a generic
        `tablet_updated` notice).
      - OR fetch lazily on each idle-hero render with the
        ceremony service's in-process API (no LLM call, just a
        SQL query).
      Implementer's choice; document the choice in the status
      update.
- [ ] Brief render uses the same markdown render path as user
      messages (`crate::markdown::render_markdown` or the
      equivalent) — no custom widget, no separate styling. The
      brief should look identical whether shown via `/brief` or
      auto-rendered in empty chat.
- [ ] Snapshot tests:
      - `snapshot_idle_hero_with_brief` — transcript empty,
        daily tablet present, weekly tablet present → renders
        the brief.
      - `snapshot_idle_hero_pre_onboarding` — transcript empty,
        no tablets → renders the T-0331 welcome (the existing
        snapshot, untouched).
      - `snapshot_idle_hero_partial_brief` — transcript empty,
        only daily present → renders the brief with the
        weekly-missing placeholder.
- [ ] Manual verification:
      - `cargo run --bin arawn -- tui` on a fresh data dir → see
        the welcome (pre-onboarding state).
      - After firing `/day` or letting the cron produce a
        tablet → reopening empty chat shows the brief.
      - Typing `/brief` mid-conversation shows the same brief
        as a system message.
- [ ] `angreal test unit` green. `angreal check workspace` green.

## Implementation Notes

### Technical Approach

1. **Slash-command registry.** Find where `/connect`, `/day`,
   `/diag` etc. are wired — same place gets `/brief`. The
   handler is straightforward: call ceremony service for the
   two tablets, build view, render, append as system message.
2. **Ceremony service API.** Look at the existing
   `daily_current` / `weekly_current` tools in
   `arawn-engine::tools::daily` / `arawn-engine::tools::weekly`
   for the data-access pattern. The slash command runs on the
   TUI side and probably has its own path; mirror it.
3. **Idle-hero probe.** The hero already runs on the TUI side
   with access to `App` state. Cache `daily_brief:
   Option<DailyView>` and `weekly_brief: Option<WeeklyView>`
   on `App`. Populate on:
   - Session start (one-shot fetch).
   - WebSocket notice that a tablet was generated (if such a
     notice exists today; otherwise file Phase 4 to add it and
     do a polling-on-render fallback now).
4. **Render path.** Empty-chat brief renders via the same
   markdown pipeline used for assistant messages. No new
   widget. The hero box drops once the brief renders.

### Dependencies

- [[ARAWN-T-0349]] — the composer this task wires up.
- Existing slash-command registry, ceremony service, idle-hero
  render path.

### Risk Considerations

- Cached tablets going stale: if a new daily tablet generates
  mid-session, the cached `App.daily_brief` won't reflect it
  until the next session unless we wire a refresh. Acceptable
  for Phase 2; Phase 4 (notification surface) is the right
  place to land `briefing_ready` toasts that trigger a refresh.
- Slash-command output rendering: appending a long markdown
  doc as a system message could overflow the transcript view.
  Should fit within existing wrap pipeline; the daily+weekly
  composite is ~30-60 lines on a real day, well under
  conversation length limits.
- Empty-chat brief rendering should NOT spend any LLM tokens
  — purely a read-and-render. Confirm the ceremony fetch
  methods are LLM-free SQL queries before wiring.

## Status Updates

### 2026-05-19 — /brief wired + empty-chat auto-render shipped

- **Slash command.** Registered `/brief` in
  `CommandRegistry::register_builtins`; added
  `CommandResult::BriefShow` + dispatch in `app.rs` and
  `event_loop.rs`, mirroring the existing `CeremonyShowToday/
  Week/Retro` pattern. Result is a system message with the
  composed brief markdown.
- **Fetch helpers.** Two new helpers in `event_loop.rs`:
  - `fetch_daily_view(client, today) -> Option<DailyView>`
  - `fetch_weekly_view(client, iso_week) -> Option<WeeklyView>`
  Existing `render_ceremony_today/week` left intact for
  `/today` / `/week`; `/brief` composes via the new helpers.
- **Composer.** New `render_brief_combined(client) -> String`
  fetches both views, builds `BriefView`, hands to
  `arawn_ceremonies::render_brief` (from [[ARAWN-T-0349]]).
- **Empty-chat auto-render.**
  - `App.brief_markdown: Option<String>` added; populated once
    at session start (right after the skill-cache fetch in
    `run_tui`) via `render_brief_combined`. The cache only sets
    when at least one tablet exists — when both sections are
    placeholders, we leave `brief_markdown = None` so the
    pre-onboarding T-0331 welcome wins.
  - `App::should_show_brief_in_empty_chat()` predicate gates
    the swap.
  - `render_chat` branches: brief if cached & non-empty, else
    `render_idle_hero` (existing welcome).
  - New `render_empty_chat_brief` renders the markdown via
    `markdown::markdown_to_lines_with_width` — the same
    pipeline assistant messages use, so the brief inherits the
    Catppuccin Mocha styling I-0036 wired up.
- **Re-export.** `arawn_ceremonies` lib now re-exports
  `BriefView` + `render_brief` alongside the existing views.
- **Snapshot tests.**
  - `snapshot_idle_hero` — extended with `app.brief_markdown =
    None` so the existing snapshot is now the explicit
    pre-onboarding guard.
  - `snapshot_idle_hero_with_brief` — new: brief markdown
    cached, empty chat renders the brief (date + Today +
    Calendar bullet + This week + Priorities bullet).
  - `snapshot_idle_hero_partial_brief_only_placeholders` — new:
    brief cached with a weekly-missing placeholder; captures
    placeholder rendering.
- **Freshness:** decided to fetch once at session start.
  Mid-session refresh belongs in Phase 4 (`briefing_ready`
  ServerNotice toasts); for Phase 2 a fresh session reads
  fresh tablets, which matches user-facing reality (cron fires
  pre-7am; user opens arawn after, sees today's brief).
- `cargo test -p arawn-tui --lib`: 182 passed (180 prior + 2
  new). `angreal test unit`: workspace green.
  `angreal check workspace` green.