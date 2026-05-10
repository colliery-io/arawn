---
id: slack-thread-cursor-regresses-to
level: task
title: "slack thread cursor regresses to parent_ts when no new replies"
short_code: "ARAWN-T-0231"
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

# slack thread cursor regresses to parent_ts when no new replies

## Parent Initiative

[[ARAWN-I-0039]]

## Severity

P1 — independent of T-0226 (cron_recovery) and T-0230 (duplicate schedules), this bug alone causes the slack/channel-archive feed to re-pull every reply on every cron tick after the first. Found during T-0218 UAT after T-0226 + T-0230 were both fixed and the duplication continued.

## Reproduction

1. Register a slack/channel-archive feed against any channel that has at least one thread with replies.
2. Let it run twice.
3. Inspect any `threads/<parent_ts>.jsonl` file — every reply will appear twice. Three runs → three times. Etc.

## Root cause

In `RealSlackClient::thread_replies` (`crates/arawn-feeds/src/clients/slack.rs:284`), the next cursor is computed as:

```rust
let next_cursor_ts = messages
    .iter()
    .filter_map(|m| m.get("ts").and_then(|v| v.as_str()))
    .max()
    .map(str::to_string)
    .or_else(|| oldest_ts.map(str::to_string));
```

`max(message.ts)` over the messages Slack returned. Slack's `conversations.replies` **always returns the parent message** as the anchor, regardless of the `oldest` filter (this is a documented Slack contract). When `oldest` is set to a reply ts and no replies match (i.e. the thread is caught up), the response is `[parent]` only — and `max([parent.ts]) = parent_ts`, which is **less than** the prior cursor (since replies are newer than the parent).

The cursor therefore regresses to parent_ts. Next call uses `oldest=parent_ts` which matches every reply (replies > parent in time) → Slack returns parent + all replies → we write the replies all over again.

Cycle:
- iter N (cursor=latest_reply): Slack returns [parent] → cursor regresses to parent_ts.
- iter N+1 (cursor=parent_ts): Slack returns parent + N replies → writes all N → cursor → latest_reply.
- iter N+2: same as iter N.
- Infinite re-pull at each cron tick.

## Confirming evidence

```
$ jq -r .ts threads/1774878412.245989.jsonl | sort | uniq -c | sort -rn | head
   8 1774894645.742199
   8 1774890015.349479
   8 1774888423.744599
   ... (every reply 8x)

$ jq '.cursor.threads["1774878412.245989"]' meta.json
"1774878412.245989"   # ← parent_ts, NOT a reply ts
```

Cursor sitting at parent_ts is the smoking gun.

## Fix landed

Two-line change in `crates/arawn-feeds/src/templates/slack/common.rs::archive_channel_with_threads` pass-2 cursor update — only advance the cursor if it's strictly greater than the prior:

```rust
if let Some(new_cursor) = page.next_cursor_ts {
    let should_advance = match prior.as_deref() {
        Some(prior_ts) => new_cursor.as_str() > prior_ts,
        None => true,
    };
    if should_advance {
        threads_cursor.insert(parent_ts.clone(), Value::String(new_cursor));
    }
}
```

The first iteration (prior=None) always advances. Subsequent iterations only advance when the new cursor is strictly greater than the old one. When Slack returns just the parent, `next_cursor_ts == parent_ts < prior_ts`, the guard fires and the cursor stays put.

## Acceptance Criteria

- [x] Cursor never regresses for a slack thread once it's been advanced.
- [x] All 11 existing `slack_channel_archive` tests still pass.
- [ ] Add a regression test: run the helper twice with a mock that returns parent+replies on first call and parent-only on second; assert cursor unchanged after second call and no new lines written. (v2 test gap — landed the fix during UAT to stop the bleeding.)

## Status Updates

### 2026-05-10 — fixed during UAT

Caught during continued UAT after T-0226 (cron_recovery upstream fix) and T-0230 (duplicate schedules) were both landed. Despite both prior fixes, slack thread files continued accumulating duplicates. Diagnosed via direct meta.json inspection — cursor for a thread was sitting at parent_ts (less than any reply ts), proving cursor regression.

This was previously misdiagnosed in T-0228 as "Slack's `oldest` parameter is inclusive on `conversations.replies`". The actual root cause is the cursor regression here. T-0228's defensive `ts <= floor` boundary skip is still in place and harmless — it just doesn't fire under normal advance-only operation.