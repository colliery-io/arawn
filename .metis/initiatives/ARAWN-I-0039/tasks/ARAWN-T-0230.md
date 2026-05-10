---
id: register-one-accumulates-duplicate
level: task
title: "register_one accumulates duplicate cron schedules per server restart"
short_code: "ARAWN-T-0230"
created_at: 2026-05-10T00:00:00+00:00
updated_at: 2026-05-10T00:00:00+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: true
initiative_id: ARAWN-I-0039
---

# register_one accumulates duplicate cron schedules per server restart

## Parent Initiative

[[ARAWN-I-0039]]

## Severity

P1 — found during T-0218 UAT. Compounded the cron_recovery firing-storm in T-0226 by an additional 7x.

## What was happening

`register_one` in `crates/arawn-feeds/src/runtime.rs` calls `runner.register_cron_workflow(&workflow_name, ...)`. cloacina's `register_cron_workflow` always inserts a new row in the `schedules` table — there is no upsert.

Boot-time scan in `arawn_feeds::start` calls `register_one` for every enabled feed on every restart. After 7 server restarts during the UAT session, every feed had 7 cron schedules with the same `workflow_name`. Sample from `~/.arawn/workflows.db`:

```
feed_API              | 7
feed_me               | 7
feed_my               | 7
feed_sd               | 7
feed_yeti-letters     | 7
feed_domino-data-labs | 5
```

Every cron tick fires every schedule whose `next_run_at` has come due. With 7 schedules per workflow, every */30 boundary spawned 7 workflow executions of the same feed simultaneously. Each execution does its own Slack/Confluence/Jira call, writes its own JSONL lines, advances its own cursor — pointed at the same `meta.json`. Result: 7x duplicate disk writes + 7x provider hammering per cron tick, on top of T-0226's cron_recovery loop.

## Fix landed

Two-line change in `register_one`:

```rust
// Idempotency: cloacina's register_cron_workflow always inserts a
// new row — no upsert. So every register_one call would add a
// duplicate schedule for the same workflow_name. Clean up first.
delete_schedule_for(runner, &workflow_name).await?;
runner
    .register_cron_workflow(&workflow_name, &record.cadence, "UTC")
    .await?;
```

Mirrors what `pause_feed` and `remove_feed` already do via the same `delete_schedule_for` helper.

Also bulk-deduplicated the existing schedule rows during UAT remediation:

```sql
DELETE FROM schedules
WHERE id NOT IN (
  SELECT id FROM (
    SELECT id, ROW_NUMBER() OVER (PARTITION BY workflow_name
                                  ORDER BY created_at DESC) AS rn
    FROM schedules WHERE workflow_name LIKE 'feed_%'
  ) WHERE rn = 1
)
AND workflow_name LIKE 'feed_%';
```

Verified post-restart: each `feed_*` workflow now has exactly one schedule, even across multiple restarts.

## Acceptance Criteria

- [x] `register_one` calls `delete_schedule_for(workflow_name)` before `register_cron_workflow`.
- [x] Existing duplicate schedules in workflows.db cleaned up.
- [x] Verified single schedule per workflow_name after server restart on the fix.
- [ ] Add a regression test that calls `register_one` twice for the same feed and asserts only one row in `schedules`. (Filed as a v2 test gap — fix landed during UAT to stop the bleeding.)

## Status Updates

### 2026-05-10 — fixed during UAT

Found while diagnosing why a fresh `/watch domino-data-labs since=180d` produced three competing cron firings at the next */15 boundary. Three of the seven historical schedules for `feed_domino-data-labs` survived the prior `/feeds rm` calls (presumably because earlier code paths missed the `delete_schedule_for` call somehow, or earlier remove_feed bugs). Plus the spawn-loop's `register_one` would have added an 8th once the backfill completed.

Fix landed (two lines in register_one). Bulk-cleaned the historical dupes via the SQL above. Server restart confirms single schedule per workflow now.

This was a real load multiplier on top of T-0226's cron_recovery bug — together they explain the 33-37x firing amplification observed earlier in the UAT session.