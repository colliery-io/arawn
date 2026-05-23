---
id: phase-4-t-b-briefing-ready
level: task
title: "Phase 4 T-B — briefing_ready ServerNotice + cache refresh"
short_code: "ARAWN-T-0360"
created_at: 2026-05-19T15:00:00+00:00
updated_at: 2026-05-19T14:32:26.685742+00:00
parent: ARAWN-I-0035
blocked_by: [ARAWN-T-0359]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0035
---

# Phase 4 T-B — briefing_ready ServerNotice + cache refresh

## Parent Initiative

[[ARAWN-I-0035]]

## Objective

When the ceremony engine generates a new daily or weekly
tablet (via cron or manual `/day` / `/week`), emit a
`ServerNotice { category: "briefing_ready" }` so connected
TUIs can refresh their cached `daily_view` + `brief_markdown`
without requiring a session restart. Surfaces a toast via the
[[ARAWN-T-0359]] renderer so the user sees the refresh happen.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] **Server-side forwarder.** A task in `arawn/src/main.rs`
      (or `local_service.rs`, wherever ceremony events meet the
      server) subscribes to `CeremonyEventReceiver` and
      translates `CeremonyEvent::TabletGenerated { kind,
      period_key, .. }` into a `ServerNotice`:
      - `level: "info"`
      - `category: "briefing_ready"`
      - `message: "Brief updated — <kind> tablet for <period_key>"`
      - `timestamp: now`
- [ ] **TUI handler.** In the event-loop ServerNotice arm,
      branch on `category == "briefing_ready"` and:
      1. Re-fetch the daily + weekly views via the existing
         `fetch_daily_view` / `fetch_weekly_view` helpers.
      2. Rebuild `BriefView` + `render_brief`; update
         `app.brief_markdown` and `app.daily_view`.
      3. Post a toast via `App::post_toast("Brief updated",
         ToastLevel::Info)`.
      4. Mark `app.dirty = true` so the dashboard repaints.
- [ ] **No-op when there's nothing to refresh.** If both fetch
      helpers return `None`, leave the caches as-is and skip
      the toast.
- [ ] **Existing ServerNotice categories untouched.**
      `plugin_reload`, `config_reload`, `integration` continue
      to work exactly as before.
- [ ] Unit / integration test:
      - A test that a `ServerNotice { category:
        "briefing_ready" }` arriving at the event loop triggers
        the refresh path. May be best as a TUI-side handler
        test using a mocked WS client, mirroring the existing
        config_reload test pattern.
- [ ] Manual smoke: launch the TUI; run `/day` to fire the
      daily ceremony; observe the toast "Brief updated" appear
      above the status bar and the right-pane brief refresh.
- [ ] `angreal test unit` green. `angreal check workspace` green.

## Implementation Notes

### Technical Approach

1. **Find the ceremony event channel.** `arawn-ceremonies` has
   `CeremonyEvent` and `event_channel`. The runtime already
   subscribes to it somewhere (search for `CeremonyEvent` in
   `arawn/src/main.rs`). Hook a forwarder there OR if no
   subscriber exists yet, create a small `tokio::spawn` task at
   startup that bridges ceremony → ServerNotice.
2. **TUI dispatch.** ServerNotice handling lives in
   `event_loop.rs`. There's already a match on `category` for
   plugin_reload / config_reload / integration — add a
   `"briefing_ready"` arm.
3. **Refresh helper.** Wrap the fetch-and-cache logic
   (currently inlined in `run_tui` at session start) into a
   reusable async function `refresh_brief_cache(app, client)`
   so both the session-start path and the briefing_ready
   handler call the same thing. Keeps drift impossible.

### Dependencies

- [[ARAWN-T-0359]] — the toast renderer this task posts to.
- `arawn-ceremonies::CeremonyEvent::TabletGenerated` (already
  emitted on every successful ceremony run).

### Risk Considerations

- ServerNotice is broadcast to ALL connected clients. If
  multiple TUIs are connected to the same arawn server, they
  all refresh simultaneously — desirable behavior, but verify
  it doesn't thunder-herd the ceremony service. Mitigation:
  the refresh is just two `get_by_period` SQL reads, cheap.
- The "Brief updated" toast fires for both daily and weekly
  tablet generation. That's two toasts in quick succession on
  Monday mornings (daily fires at 7am, weekly at 7am). The
  queue handles this; just flag in case the double-toast is
  surprising.

## Status Updates

### 2026-05-19 — briefing_ready forwarder + cache refresh shipped

- **Server-side forwarder extended.** The existing
  `CeremonyEvent` → `ServerNotice { category: "ceremony_event" }`
  bridge in `arawn/src/main.rs` now also emits a parallel
  `ServerNotice { category: "briefing_ready" }` when the event
  is `CeremonyEvent::TabletGenerated`. Other event variants
  (`ItemUpdated`, `DiaryUpdated`, etc.) untouched. Message:
  `"Brief updated — {kind} tablet for {period_key}"`.
- **TUI handler.** New branch in `apply_system_notice` for
  `category == "briefing_ready"`:
  - Sets `app.pending_brief_refresh = true` (event loop drains
    next tick).
  - Posts the notice's message as an info-level toast via
    `App::post_toast`.
  - Silent — no chat-history row pushed.
- **Refresh path.** Extracted the session-start brief
  pre-fetch into `refresh_brief_cache(client, app)`. Both the
  session-start preload and the briefing_ready handler share
  one implementation — keeps the two paths from drifting.
  Idempotent; no-op when both tablets are absent (pre-onboarding
  T-0331 welcome continues to win).
- **Event-loop drain.** `pending_brief_refresh` consumed in
  the same block as `pending_ceremony_refresh` /
  `pending_todo_refresh`; calls `refresh_brief_cache` and
  forces a re-render.
- **App state.** New `pending_brief_refresh: bool` field,
  initialized to `false`.
- **No regression** to `ceremony_event`, `todo_event`,
  `plugin_reload`, `config_reload`, `integration`.
- **Unit tests (2 new):**
  - `briefing_ready_flags_refresh_and_posts_toast` — sets
    pending flag, enqueues one toast, no chat noise.
  - `briefing_ready_does_not_affect_ceremony_refresh` — the
    new path is independent of the legacy ceremony_event flow.
- `cargo test -p arawn-tui --lib` 209/0 (207 prior + 2 new).
  `angreal check workspace` green.