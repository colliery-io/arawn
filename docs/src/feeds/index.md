# Continual Data Feeds

A **feed** in arawn is a recurring ingest job that mirrors a slice of
upstream state (a Slack channel, a Gmail label, a Drive folder, a Jira
project) into your local arawn data directory. Once a feed is running,
the agent doesn't have to call out to the provider every time you ask
about that data — it reads the local mirror with the same `Read`,
`Grep`, and `Glob` tools it uses on source code.

This is the substrate for "ask arawn about my world" — what's happening
in #design today, what's on my plate in ENG, did boss@x.com email me
this week.

## When feeds make sense

| Use a feed when... | Skip the feed when... |
|---|---|
| You'll ask about the data repeatedly | One-off lookup is fine |
| You want history beyond the API's default window | The provider's API already does what you need |
| You want grep / glob over the data | A direct tool call is faster |
| Multiple agent runs benefit from the same fetch | The data changes faster than the cadence |

Feeds aren't a replacement for tools like `gmail_search` or
`slack_search_messages` — they're a complement. The tools answer
"go fetch this *now*"; feeds answer "I already have a local snapshot,
let me grep it."

## What lands where

Every feed writes under one root:

```
~/.arawn/data/<provider>/<template>/<feed_id>/
  ├── meta.json           # runtime-managed cursor + last-run status
  └── <template's data>   # JSON files, JSONL logs, mirrored bodies
```

- `<provider>` is `slack`, `gmail`, `calendar`, `drive`, `jira`, or
  `confluence`.
- `<template>` is the recipe (`channel-archive`, `inbox-archive`,
  `folder-sync`, ...).
- `<feed_id>` is your handle for this instance (`design` for the
  #design channel, `ENG` for the ENG Jira project, etc.).

`meta.json` is owned by the runtime — it tracks the cursor (where the
next run resumes from), `last_run_at`, `last_status`, and `run_count`.
Everything else under the feed dir is the template's territory.

The data dir's layout is the public contract feeds publish to the
agent. The `template-catalog.md` page documents the exact on-disk
shape per template.

## How a feed gets created

Two ways:

1. **Auto-create on `/connect`.** Connecting an integration that has a
   "personal" feed registers it automatically — e.g. `/connect gmail`
   sets up a `gmail/inbox-archive` feed for "me", and `/connect
   google_drive` registers `drive/recent`. You don't have to think
   about cadence or params.
2. **`/watch <template> <feed_id> [params]`** for everything else.
   Pick the channel, project, folder, sender — whatever the template
   needs. Optional `since=<rfc3339>` triggers a one-time backfill
   before the cron schedule starts (see "Backfill mode" below).

`/watch list <template>` shows what's available for a template you've
connected — e.g. `/watch list slack/channel-archive` lists every
channel the bot can see, including private ones it's been invited to.

`/feeds` lists everything currently running. `/unwatch <feed_id>`
removes a feed and (optionally) its data.

## Cadences

Each template ships with a sensible default cron schedule (most are
every 15 or 30 minutes). You can override with `cadence=<cron>` at
`/watch` time if you have a reason — but the defaults are tuned to
the providers' rate limits and how fresh the data realistically needs
to be.

| Template family | Default cadence |
|---|---|
| Slack (real-time-ish) | every 15 min |
| Gmail (mostly current) | every 15–30 min |
| Calendar / Jira / Confluence / Drive recent | every 30 min |
| Drive folder-sync / Slack DM-archive | hourly |

Cron expressions use standard 5-field syntax (`*/15 * * * *`).

## Backfill mode

When you pass `since=<rfc3339>` to `/watch`, arawn runs a one-shot
backfill loop before registering the cron schedule. It walks the
provider's pagination from `since` forward, persisting the cursor
after every page so a server restart resumes from where it left off.
Once caught up, the row flips to `enabled=1` and cron takes over.

If the backfill hits a rate limit it can't drain within 5 minutes of
cumulative waits, it stops gracefully, flags `last_status =
"backfill-rate-limited"`, and lets the next cron tick continue from
the persisted cursor. Same for transient errors — three retries with
exponential backoff before bailing.

This means a 6-month Gmail backfill or a 5000-issue Jira pull is a
single command that just works, even across rate-limit waves.

## Reading the data

The agent reads feed data with the same tools it uses on code:

- `Read` for a single JSON snapshot — `~/.arawn/data/jira/project-tracker/ENG/ENG-742/issue.json`
- `Glob` for "everything matching a shape" — `~/.arawn/data/slack/channel-archive/design/2026-05-*/messages.jsonl`
- `Grep` for content search — `grep -l "deploy" ~/.arawn/data/gmail/inbox-archive/me/2026-*/`

See `agent-read-patterns.md` for worked examples — actual prompts that
exercise these patterns.

## Disk usage

Feeds are local-first by design — there's no upstream summarizer, no
cloud index. That keeps you sovereign over your data but means disk
grows over time. Rough guides:

- Slack channel-archive: ~1–5 MB per active channel per month
- Gmail inbox-archive: ~10–50 MB per month for a typical inbox
- Drive folder-sync: scales with the folder (mirrors bodies)
- Drive recent: metadata-only, ~1 MB / month
- Jira / Confluence: depends on project size; usually small

If a feed grows unexpectedly, check the per-day partitions — usually
one runaway day (a noisy bot, a backup job) is the cause.

## When things go wrong

`meta.json.last_status` tells you the last run's outcome:

| Status | Meaning |
|---|---|
| `ok` | Wrote new items |
| `no-new-items` | Ran clean, nothing new to fetch |
| `backfill-rate-limited` | Backfill hit the 5-min cap; cron will resume |
| `backfill-failed: <reason>` | Backfill couldn't recover; manual look needed |
| `auth failed: ...` | Provider token revoked / scope removed → `/connect` again |

The runtime is conservative: provider errors don't crash arawn, they
get logged and the next cron tick tries again. A single bad item
(malformed Gmail message, Drive file with no body) is skipped, not
fatal — see T-0237's schema-skip behaviour.

## Where next

- [Template catalog](./template-catalog.md) — every template, every
  param, every on-disk layout.
- [Agent read patterns](./agent-read-patterns.md) — worked examples
  of asking the agent about feed data.
- The [Integrations](./../integrations/) section covers connecting
  each provider in the first place.
