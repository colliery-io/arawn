---
id: runtime-hot-register-of-new-feeds
level: task
title: "Runtime hot-register of new feeds on bind"
short_code: "ARAWN-T-0329"
created_at: 2026-05-18T18:32:39.412152+00:00
updated_at: 2026-05-18T18:50:26.991469+00:00
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

# Runtime hot-register of new feeds on bind

## Objective

When a `/workstream bind` registers a new feed (today: any
`github:repo:` scope binding; tomorrow: any feed registered via
a tool or RPC), the cloacina runtime should pick it up
immediately — not at the next process restart.

I-0050 left this as a known follow-up:

> Cron pickup of the new feed records relies on the feed-
> runtime's existing startup re-scan; runtime hot-add of a
> brand-new feed_id is not wired here. Worst case: a newly-
> bound repo's first poll waits until the next process restart,
> after which it's on the 30-min cadence.

## Acceptance Criteria

- [x] `FeedRuntime::unregister_cron(feed_id)` — wraps the
      existing private `delete_schedule_for` helper. Surgical
      cron-only teardown (no DB row delete, no feed_dir wipe).
      Idempotent.
- [x] Repo bind: `ExtractorBindHook::on_bind` spawns
      `register_one_feed` after firing the backfill. That
      helper fetches the just-inserted feed record via
      `FeedStore::get` and calls
      `FeedRuntime::register_feed_runtime(&record)`. Cron is
      live within a tick of the bind.
- [x] Org bind: `expand_github_org` collects newly-inserted
      feed_ids from `INSERT OR IGNORE` row counts and registers
      cron for each via the same helper. Existing entries
      skipped.
- [x] Unbind teardown: new `UnbindHook` trait in arawn-engine
      with `WorkstreamUnbindTool::with_unbind_hook` builder.
      The tool collects every feed_id removed from the feeds
      table (single-feed for repo, LIKE-prefix-collected for
      org) and fires the hook outside the store lock.
- [x] Late-bound `Arc<RwLock<Option<Arc<FeedRuntime>>>>` cell
      threads the live runtime from `arawn_feeds::start` back
      into the bind + unbind hooks (constructed earlier).
- [x] No regressions to existing `/watch` flow:
      `register_feed_runtime` is the same code path
      `register_feed_dynamic` already used.
- [x] 3 new unit tests: hook captures single feed_id on repo
      unbind, all child feed_ids on org unbind (sorted check),
      no-op for non-github bindings.
- [-] End-to-end live-runtime regression test deferred: would
      need a cloacina runtime stub or in-memory mock that
      records `register_cron_workflow`. The hook-state tests
      prove the trigger semantics; the cron call itself is
      exercised through the existing `register_one` path used
      by `/watch`.

## Implementation Notes

### Technical Approach

Two pieces:

1. **Live cron registration**: extend `arawn-feeds::FeedRuntime`
   (or expose a new method) that takes a `FeedRecord` and
   registers the workflow + cron schedule with the running
   cloacina runner. Mirrors what `arawn_feeds::start` does at
   boot but for one feed at a time.

2. **Wire from the bind hook**: `WorkstreamBindTool`'s
   backfill-hook callback already runs after a successful
   bind. Extend the binary's `ExtractorBindHook` to also call
   `FeedRuntime::register_feed_now(rec)` (new method) for
   newly-registered feed_ids.

3. **Unbind teardown**: `WorkstreamUnbindTool` already
   removes the feeds-table row. The cron registration needs
   a companion `FeedRuntime::unregister_feed(id)` call. Add
   the symmetric method.

### Dependencies

- Touches `arawn-feeds::FeedRuntime` (new methods on the
  runtime + plumbing through `start`'s loop).
- Wired from `crates/arawn/src/main.rs` (the bind hook).
- Touches `WorkstreamUnbindTool` (or its caller) for the
  unregister path.

### Risk Considerations

- Concurrent register/unregister vs. an in-flight tick of the
  same feed. Cloacina's per-feed exclusivity should handle
  this, but verify before merging — a register racing with
  a tick on the previous version of the same id could leave
  the schedule in an inconsistent state.
- The `register_feed_now` API exposes a wider surface than the
  existing one-shot boot loop. Keep the entry point narrow
  (single `FeedRecord` in, success/error out) to discourage
  callers from re-implementing boot-time logic.

## Status Updates

### 2026-05-18 — hot register/unregister wired

- `FeedRuntime::unregister_cron` is the surgical counterpart to
  `register_feed_runtime`. Both no-op gracefully when the
  schedule doesn't exist / runtime isn't wired yet.
- `register_one_feed` helper in main.rs reads the feed row from
  storage and hands it to `register_feed_runtime` — the bind
  tool keeps owning row persistence (raw SQL today), the hook
  adds the cron side. Decouples concerns cleanly.
- `UnbindHook` is symmetric to `BindBackfillHook`. The unbind
  tool collects deleted feed_ids before LIKE-deleting (org
  case) so the hook receives the actual list, not a pattern.
- Workspace 1898/0 (1895 → 1898, +3).