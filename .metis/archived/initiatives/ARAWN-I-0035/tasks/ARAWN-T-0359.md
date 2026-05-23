---
id: phase-4-t-a-tui-toast-renderer-1
level: task
title: "Phase 4 T-A — TUI toast renderer (1-line ephemeral overlay)"
short_code: "ARAWN-T-0359"
created_at: 2026-05-19T15:00:00+00:00
updated_at: 2026-05-19T14:28:04.560571+00:00
parent: ARAWN-I-0035
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0035
---

# Phase 4 T-A — TUI toast renderer

## Parent Initiative

[[ARAWN-I-0035]]

## Objective

Add a 1-line ephemeral toast surface to the TUI that any
`ServerNotice` handler can call to surface a passing message
(e.g., "Brief updated", "Slack token refreshed", "Workflow X
produced 3 findings"). Universal infrastructure — the consumers
([[ARAWN-T-0360]] briefing_ready, future watcher_finding /
integration_health) plug into the same API.

Toasts render as a single styled line above the status bar.
Decay after ~5 seconds. Queueable: if multiple toasts arrive
in quick succession, render the newest and stack the rest for
sequential display.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] New `App.toast_queue: VecDeque<Toast>` field carrying
      `{ message: String, level: ToastLevel, posted_at: Instant,
      ttl: Duration }`. `ToastLevel` enum: `Info | Warn | Error`
      mapping to existing theme colors (SUBTEXT0 / YELLOW / RED).
- [ ] `App::post_toast(message, level)` helper enqueues a new
      toast with the default TTL (`TOAST_TTL` const, ~5s).
- [ ] Render path: when the queue is non-empty AND the front
      toast's `posted_at + ttl > now`, render a 1-line widget
      above the status bar using the toast's level styling. When
      the front toast expires, pop it and continue.
- [ ] Place the toast row in the existing vertical layout right
      above the status bar (or above the OAuth heartbeat if that
      row is active). Add the constraint conditionally so we
      don't waste a row when no toast is queued.
- [ ] No focus / click interaction — the toast is purely
      informational. Toasts dismiss themselves on TTL expiry; no
      manual close key.
- [ ] Frame ticking: the event loop already drives `Action::Tick`
      regularly (for spinners). Hook the existing tick path to
      refresh the toast surface when a toast is queued — no new
      timer needed.
- [ ] Wrap measurement: the toast message is truncated with
      `…` if it exceeds the terminal width (use
      `crate::width::truncate_display`).
- [ ] Unit tests:
      - `post_toast_enqueues_message` — `App::post_toast` adds
        to the queue.
      - `expired_toast_is_dropped_on_render` — after TTL elapses,
        the front toast is popped.
      - `toast_truncates_long_message`
- [ ] Snapshot test:
      `snapshot_toast_visible` — three-pane layout + a toast
      enqueued, captures the toast row above the status bar.
- [ ] `angreal test unit` green. `angreal check workspace` green.

## Implementation Notes

### Technical Approach

1. **`Toast` + `ToastLevel`** structs live in `arawn-tui::app`
   (or a new `toast.rs` module under `arawn-tui::src/`). Small
   enough that a single file is fine.
2. **App state:** add `pub toast_queue: VecDeque<Toast>` next to
   the other UI-state fields. Initialize empty in `App::new`.
3. **Render placement:** in `render.rs::render`, after computing
   the vertical layout but before splitting horizontals, peek
   `app.toast_queue.front()` — if present and not expired, add
   a `Constraint::Length(1)` row to the vertical layout right
   above the status bar. `render_toast_bar(app, frame, area)`
   draws the front toast.
4. **Expiry:** lazy — when `render_toast_bar` is called, it
   pops expired toasts off the front and renders the first
   non-expired one (or nothing). No background task.
5. **Level → style:**
   - `Info` → `theme::SUBTEXT0` foreground, bold.
   - `Warn` → `theme::YELLOW`, bold.
   - `Error` → `theme::RED`, bold.
6. **Tick-driven repaint:** the existing tick handler in
   event_loop sets `app.dirty = true` periodically. Add a check:
   if `!app.toast_queue.is_empty()`, also set dirty so the
   toast disappears on TTL even when nothing else changes.

### Dependencies

- `arawn-tui` only. ServerNotice handlers (T-0360 onward) will
  call `App::post_toast` — they don't need to know about the
  rendering details.

### Risk Considerations

- Multiple toasts in quick succession can pile up; the queue
  may grow if the agent never gets a tick. Cap the queue at a
  small N (e.g. 8) and drop the oldest when full — a sane
  default that won't surprise anyone.
- The toast row consumes 1 cell of vertical space. On very
  short terminals (≤ 12 rows) this might displace useful chat
  content. Acceptable — the row only exists when a toast is
  active, and a toast IS useful info during that window.

## Status Updates

### 2026-05-19 — Toast renderer shipped

- **New `arawn-tui::toast` module** with `Toast { message, level,
  posted_at, ttl }`, `ToastLevel { Info | Warn | Error }`, plus
  free functions `drop_expired` and `enqueue`. Constants:
  `TOAST_TTL = 5s`, `TOAST_QUEUE_CAP = 8` (oldest dropped when
  full).
- **`App.toast_queue: VecDeque<Toast>`** added. New
  `App::post_toast(message, level)` enqueues + marks the app
  dirty so the next render picks it up.
- **Vertical layout extended:** added a conditional 1-row
  constraint between input and OAuth heartbeat. Layout indices
  shifted accordingly (status bar now at `vertical[5]`).
- **`render_toast_bar`** renders the head-of-queue toast with
  the level-mapped color (SUBTEXT0 / YELLOW / RED), bold,
  truncated via `crate::width::truncate_display` to fit row
  width.
- **Lazy expiry:** `drop_expired(queue, Instant::now())` runs
  at the start of every `render` call so an expired toast can't
  remain visible past its TTL.
- **Tick-driven repaint:** the event-loop tick handler now also
  redraws when `!app.toast_queue.is_empty()` so toasts age out
  even when nothing else is happening.
- **Unit tests:**
  - In `toast.rs`: `is_expired_after_ttl`,
    `is_not_expired_within_ttl`, `drop_expired_pops_front_only`,
    `enqueue_caps_at_max`.
  - In `render.rs::tests`: `post_toast_enqueues_message`,
    `expired_toast_is_dropped_on_render`,
    `toast_truncates_long_message`.
- **Snapshot test:** `snapshot_toast_visible` — three-pane
  layout with a toast row above the status bar.
- `cargo test -p arawn-tui --lib` 207/0 (199 prior + 8 new).
  `angreal check workspace` green.