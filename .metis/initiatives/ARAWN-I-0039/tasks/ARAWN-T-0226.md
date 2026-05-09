---
id: cloacina-0-6-cron-recovery
level: task
title: "cloacina 0.6 cron_recovery feedback loop — schedule_executions never marked complete"
short_code: "ARAWN-T-0226"
created_at: 2026-05-09T00:00:00+00:00
updated_at: 2026-05-09T00:00:00+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0039
---

# cloacina 0.6 cron_recovery feedback loop

## Parent Initiative

[[ARAWN-I-0039]]

## Objective

**Severity: P1 — bug found during T-0218 live UAT.** Without the workaround applied in this task's first sub-fix (disabling `cron_enable_recovery`), every cron-scheduled feed re-fires every ~13 seconds instead of on its declared cadence. With the slack/channel-archive feed registered at `*/15 * * * *`, we observed **906 runs in 6 hours** — should have been ~24. Each spurious run hammers the upstream API (Slack rate-limited the thread-replies sub-fetches), accumulates duplicate work-in-flight, and is generally catastrophic for any feed that ships out a real network call.

This task fixes the root cause cleanly so we can re-enable `cron_enable_recovery` (which is what catches missed firings across server restarts).

## Reproduction (pre-workaround)

1. Boot arawn with `cron_enable_recovery(true)` (the default before this task's sub-fix).
2. Register any feed: `/watch slack/channel-archive design channel=#anything`.
3. Watch the server log for ~5 minutes.
4. Observe: a single cron firing produces 30+ "feed run complete" log lines spaced ~13s apart. The cron schedule's `next_run_at` advances correctly to the next 15-min boundary, but `cron_recovery` keeps re-scheduling the workflow.

## Root cause

Confirmed via direct sqlite inspection of `~/.arawn/workflows.db`:

```sql
SELECT scheduled_time, claimed_at, started_at, completed_at
FROM schedule_executions ORDER BY scheduled_time DESC LIMIT 5;

scheduled_time            | claimed_at                  | started_at                  | completed_at
2026-05-09T10:45:00+00:00 | 2026-05-09T10:45:14.014128  | 2026-05-09T10:45:14.014244  | (empty)
2026-05-09T10:30:00+00:00 | 2026-05-09T10:30:29.010102  | 2026-05-09T10:30:29.010167  | (empty)
2026-05-09T10:15:00+00:00 | 2026-05-09T10:15:15.009614  | 2026-05-09T10:15:15.009912  | (empty)
```

The `schedule_executions` row's `completed_at` column is **never populated** when the workflow finishes successfully. Workflow audit reports completion ("Workflow execution completed: ... 1 completed, 0 skipped") but the schedule_execution row is left in `started_at-but-not-completed` state.

cloacina's `CronRecoveryService::check_and_recover_lost_executions()` then queries `find_lost_executions(threshold_minutes)`. By default, anything past the threshold with a `started_at` but no `completed_at` is considered "lost" and gets re-scheduled. The default check interval is 300s but executions appear lost after a short threshold, so on every recovery cycle each prior execution gets re-fired. The result is the observed feedback loop.

## Workaround in place

`crates/arawn-workflow/src/runner.rs` now calls `.cron_enable_recovery(false)` on the runner config. Trade-off: feeds will no longer auto-recover from missed firings (e.g. server crashed during a scheduled run). Acceptable for current usage — feeds are continual and the next regular cron tick will pick up where the cursor left off.

## Acceptance Criteria for the real fix

- [ ] Identify *which* cloacina internal is responsible for marking `schedule_executions.completed_at`. Likely candidates: `WorkflowExecutor::on_complete`, the audit pipeline, or a separate `mark_schedule_complete` hook that's never being called for embedded (non-package) workflows.
- [ ] Either:
  - File an upstream bug + minimal repro against `colliery-software/cloacina`, then patch our workflow_runner to call the missing hook, *or*
  - Patch our `arawn-feeds::dispatch::FeedDispatchTask::execute` to mark the execution complete via DAL before returning Ok (if the API allows external completion).
- [ ] Re-enable `.cron_enable_recovery(true)` in `arawn-workflow/src/runner.rs` and verify the loop is gone — drive a feed for 30+ minutes and confirm exactly N runs at the cron cadence.
- [ ] Add an integration test in `arawn-feeds/tests/dynamic_register.rs` that runs against a real `DefaultRunner` with cron_recovery enabled, registers a feed at `*/15 * * * *` (or a faster cadence the test framework allows), waits for two firings, and asserts only two executions occurred.
- [ ] Document the resolution in this task's status updates so the trade-off is recorded.

## Risk Considerations

- The recovery service exists for good reasons — server crashes, missed firings during downtime. Disabling it permanently is wrong; the proper fix is upstream.
- Workflow packaged via cloacina's macros may already mark schedules complete correctly — the missing hook may only affect embedded `register_workflow` users (us). That'd be a clean repro path: register an embedded workflow vs a packaged one and compare.

## Status Updates

### 2026-05-09 — workaround landed during T-0218 UAT

`cron_enable_recovery(false)` set in `runner.rs` to stop the bleeding. Real fix requires upstream investigation; filed as this task. The slack feed that was hammering the API has been paused via `/feeds pause` and will be resumed once we reset the cursor (it overshot to `latest_ts: 1778167136.964759` over the spurious runs, so we need to either accept those messages as legit-archived or wipe + re-register).
