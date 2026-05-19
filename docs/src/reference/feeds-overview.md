# Feeds overview

*Reference. On-disk layout, cadence, status, backfill mechanics.*

A **feed** is a recurring ingest job that mirrors a slice of upstream state (a Slack channel, a Gmail label, a Drive folder, a Jira project, a GitHub repo) to your local arawn data directory. Once a feed is running, the agent reads the local mirror with `Read`, `Glob`, and `Grep` instead of round-tripping to the provider.

For the rationale and the "when feeds make sense" framing, see [feeds explanation](../explanation/feeds.md). For the recipe to create one, see [create a feed how-to](../how-to/create-a-feed.md).

Source: `crates/arawn-feeds/`. Per-template specs: [feed templates reference](./feed-templates.md).

## On-disk layout

Every feed writes under one root:

```
~/.arawn/data/<provider>/<template>/<feed_id>/
  ├── meta.json           # runtime-managed cursor + last-run status
  └── <template's data>   # JSON files, JSONL logs, mirrored bodies
```

- `<provider>` is `slack`, `gmail`, `calendar`, `drive`, `jira`, `confluence`, or `github`.
- `<template>` is the recipe (`channel-archive`, `inbox-archive`, `folder-sync`, …).
- `<feed_id>` is your handle for this instance (`design` for the #design channel, `ENG` for the ENG Jira project).

## `meta.json`

The runtime owns `meta.json`. It tracks:

```jsonc
{
  "cursor": { ... },              // template-specific resume point
  "last_run_at": "2026-05-18T07:00:00Z",
  "last_status": "ok",            // see status table below
  "run_count": 142
}
```

The cursor shape is template-specific — see [feed templates reference](./feed-templates.md) for each template.

## Status states

`meta.json.last_status` tells you the last run's outcome:

| Status | Meaning |
|---|---|
| `ok` | Wrote new items. |
| `no-new-items` | Ran clean; nothing new to fetch. |
| `backfill-rate-limited` | Backfill hit the 5-minute rate-limit cap. Cron will resume from the persisted cursor. |
| `backfill-failed: <reason>` | Backfill couldn't recover. Manual inspection needed. |
| `auth failed: ...` | Provider token revoked or scope removed. Run `/disconnect <svc>` then `/connect <svc>`. |

The runtime is conservative: provider errors don't crash arawn — they get logged, and the next cron tick tries again. A single bad item (malformed Gmail message, Drive file with no body) is skipped, not fatal.

## Cadences

Each template ships with a sensible default cron. Override with `cadence=<cron>` at `/watch` time only if you have a reason.

| Template family | Default cadence |
|---|---|
| Slack (real-time-ish) | every 15 min |
| Gmail (mostly current) | every 15-30 min |
| Calendar / Jira / Confluence / Drive recent / GitHub | every 30 min |
| Drive folder-sync / Slack DM-archive | hourly |

Cron expressions use standard 5-field syntax (`*/15 * * * *`).

## Backfill

Pass `since=<rfc3339>` to `/watch` to backfill historical data before the cron schedule starts:

```
/watch slack/channel-archive design since=2026-01-01T00:00:00Z
```

The backfill loop walks the provider's pagination from `since` forward, persisting the cursor after every page so a server restart resumes from where it left off. Once caught up, the row flips to `enabled=1` and cron takes over.

If the backfill hits a rate limit it can't drain within 5 minutes of cumulative waits, it stops gracefully, flags `last_status = "backfill-rate-limited"`, and lets the next cron tick continue from the persisted cursor. Same for transient errors — three retries with exponential backoff before bailing.

A 6-month Gmail backfill or a 5000-issue Jira pull is a single command that just works, even across rate-limit waves.

## Inspecting feeds

```
/feeds                       # list all running feeds
/feeds pause <feed_id>       # pause cron; data stays
/feeds resume <feed_id>      # resume a paused feed
/feeds run <feed_id>         # trigger an immediate run
/feeds rm <feed_id> [--yes]  # remove the feed
```

## Disk usage

Feeds are local-first by design — no upstream summarizer, no cloud index. Rough guides:

| Template | Typical disk usage |
|---|---|
| Slack `channel-archive` | ~1-5 MB per active channel per month |
| Gmail `inbox-archive` | ~10-50 MB per month for a typical inbox |
| Drive `folder-sync` | scales with folder (mirrors bodies) |
| Drive `recent` | metadata-only, ~1 MB / month |
| Jira / Confluence | usually small; depends on project size |
| GitHub `repo-mirror` | scales with repo issue+PR activity |

If a feed grows unexpectedly, check per-day partitions — usually one runaway day (a noisy bot, a backup job) is the cause.

## Reading the data

The agent reads feed files with the same tools it uses on code — `Read`, `Glob`, `Grep`. See [read feeds with the agent how-to](../how-to/read-feeds-with-the-agent.md) for prompt patterns.

For projection-layer search across feeds, the [`feed_search` tool](./feed-search-tool.md) is the agent's structured surface.

## Related

- [Feed templates reference](./feed-templates.md) — every template with params + on-disk shape.
- [Feeds explanation](../explanation/feeds.md) — when to feed vs. just call the tool.
- [Create a feed how-to](../how-to/create-a-feed.md).
- [Read feeds with the agent how-to](../how-to/read-feeds-with-the-agent.md).
