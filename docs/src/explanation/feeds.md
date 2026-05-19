# Feeds

*Explanation. What a feed is, when to make one, why local-first, what backfill costs you.*

A **feed** is a recurring ingest job — a Slack channel, a Gmail label, a Drive folder, a Jira project, a GitHub repo. The runtime polls the provider on a cron, normalizes each new item, and writes it to disk under `~/.arawn/data/<provider>/<template>/<feed_id>/`.

For the operational mechanics, see [feeds overview reference](../reference/feeds-overview.md). For the recipe, see [create a feed how-to](../how-to/create-a-feed.md). This page is about *why* feeds exist.

## The thesis: local-first

Once a feed is running, the agent reads the local mirror instead of round-tripping to the provider. That's the whole point.

Three consequences:

1. **You stay sovereign over your data.** No upstream summarizer sees your inbox. No cloud index has a copy of your Slack history. The bytes live on your machine; the LLM provider sees only what the agent quotes into a prompt.
2. **The agent uses generic tools.** `Read`, `Glob`, `Grep` are the same tools the agent uses on source code. The feed layout is shaped so those tools are enough — day-partitioned JSONL, one-file-per-message, per-thread sub-files. No special query API; if you can grep it, the agent can grep it.
3. **Disk grows.** Feeds are a sustained write — every cron tick appends. Five years of Gmail at 50 MB/month is 3 GB. Tractable for a laptop, but not free.

## When feeds make sense

| Use a feed when... | Skip the feed when... |
|---|---|
| You'll ask about the data repeatedly | One-off lookup is fine |
| You want history beyond the API's default window | Provider's API already does what you need |
| You want grep / glob over the data | A direct tool call is faster |
| Multiple agent runs benefit from the same fetch | The data changes faster than the cadence |

Feeds aren't a replacement for tools like `gmail_search` or `jira_search`. Those answer "fetch this *now*"; feeds answer "I already have a local snapshot, let me grep it."

## What lands where

Every feed writes under one root:

```
~/.arawn/data/<provider>/<template>/<feed_id>/
  ├── meta.json           # cursor + last-run status — runtime-managed
  └── <template's data>   # template's territory
```

The data dir's layout is the public contract feeds publish to the agent. The agent navigates by convention:

- Per-day JSONL for high-volume streams (Slack channels).
- Per-message JSON for things you'll re-read individually (Gmail).
- Per-entity directory with append-only logs for ticket-shaped data (Jira issue with `comments.jsonl` + `history.jsonl`).
- Per-file native bytes for filesystem-shaped data (Drive folder-sync).

[Feed templates reference](../reference/feed-templates.md) documents the exact shape per template.

## How a feed gets created

Two paths:

1. **Auto-create on `/connect`.** Connecting an integration that has a "personal" feed registers it automatically — e.g. `/connect gmail` sets up `gmail/inbox-archive` for "me", `/connect google_drive` registers `drive/recent`. You don't think about cadence or params.
2. **`/watch <template> <feed_id> [params]`** for everything else. Pick the channel, project, folder, sender — whatever the template needs.

[Create a feed how-to](../how-to/create-a-feed.md) covers the recipe.

## Why default cadences are what they are

Each template ships with a default cron tuned to the provider's rate limits and how fresh the data realistically needs to be:

- **Slack channels**: every 15 minutes. Slack is high-volume; you don't want a 2-minute backlog of unprocessed messages, but every 5 minutes burns rate-limit budget.
- **Gmail**: every 15-30 minutes. Inbox velocity varies wildly; the cadence is "fresh enough to act on."
- **Calendar / Jira / Confluence / Drive recent / GitHub**: every 30 minutes. Lower change rate; longer windows acceptable.
- **Drive folder-sync / Slack DM-archive**: hourly. These mirror bodies (folder-sync) or low-velocity streams (DMs); hourly is plenty.

Override at `/watch` time with `cadence=<cron>` only if you have a reason. The defaults are tuned.

## Backfill: the long-tail catch-up

`/watch <template> <feed_id> since=<rfc3339>` triggers a one-shot backfill before cron takes over. The runtime walks the provider's pagination from `since` forward, persisting the cursor after every page so a server restart resumes from where it left off.

The interesting case: rate limits. A 6-month Gmail backfill at the provider's per-second rate would take hours of synchronous work. Instead:

- If a backfill exhausts a 5-minute window of cumulative waits, it stops gracefully and sets `last_status = "backfill-rate-limited"`.
- The next cron tick continues from the persisted cursor.
- Many cron ticks later, the backfill is done; the row flips to `enabled = 1` and steady-state cron takes over.

This is what makes a "6-month Gmail backfill" a single command. The runtime is conservative — slow but correct, never abandoning a partial fetch.

## Why feeds aren't transactional

Each feed run writes new files. There's no "rollback the last run" — partial writes are committed; the cursor advances only after the page is durably on disk. The system is resilient by being append-friendly: re-fetching the same `source_id` is idempotent at the projections layer (same id, same row), and the on-disk template logic handles re-writes (overwrite for mutable entities like Jira issues; skip for immutable entities like Gmail messages).

The trade-off: a malformed item doesn't crash arawn — it's skipped with a warn-level log, the cursor advances, the next tick continues. You may end up with gaps. Provider-side issues (a rare API response shape) cause similar gaps. Both are visible as `last_status` values.

## Disk usage and pruning

Local-first means you choose how much history to keep. arawn doesn't have a "prune after N days" knob today; if you want to drop old data:

```sh
rm -rf ~/.arawn/data/slack/channel-archive/design/2024-*
```

The runtime won't notice — the feed cursor is `meta.json.cursor`, which is forward-looking. The agent will just see less history.

A formal pruning subsystem (delete-old or compact-to-summary) is on the long-term roadmap but not urgent — disk is cheap, retention is your call.

## When NOT to feed

Three cases:

- **Provider's API already answers your need.** "What's the next event on my calendar?" is `calendar_upcoming`'s job; you don't need a feed for it. Feeds win when you'll grep across a large corpus.
- **Volatile data.** Live PagerDuty status doesn't make sense to mirror — by the time arawn polls, the status has changed twice. Use a direct tool call.
- **Truly private streams.** If a Slack channel is sensitive enough that you wouldn't want it on disk in a backup, don't feed it. Mirror is a multiplier on access risk.

## Related

- [Feeds overview reference](../reference/feeds-overview.md) — mechanics, statuses, backfill operational detail.
- [Feed templates reference](../reference/feed-templates.md) — the 16 user-facing templates with params.
- [Create a feed how-to](../how-to/create-a-feed.md).
- [The three-layer data model](./three-layer-data-model.md) — feeds in context.
- [Projections explanation](./projections.md) — what gets built on top.
