# Create a feed

*How-to. Make arawn mirror a slice of an external service to disk so the agent can read it locally.*

A **feed** is a recurring ingest job — a Slack channel, a Gmail label, a Drive folder, a Jira project. Once it's running, the agent answers questions about that data with `Read` / `Glob` / `Grep` against the local mirror instead of round-tripping to the provider.

## Prerequisites

- The relevant integration is connected (e.g. [connect Slack](./connect-slack.md)).
- arawn server running.

## Option A: auto-created personal feeds

Connecting Gmail, Drive, Slack, Calendar, or Atlassian auto-registers a personal feed for you:

- `/connect gmail` → `gmail/inbox-archive` for your inbox.
- `/connect google_drive` → `drive/recent` for your recent Drive activity.
- `/connect google_calendar` → `calendar/upcoming-archive` for your upcoming events.
- `/connect slack` → `slack/my-mentions` for `@you` mentions across the workspace.
- `/connect atlassian` → `jira/assignee-tracker` for `assignee = currentUser()` issues.

No further setup. After `/connect`, `/feeds` will list these as running.

## Option B: `/watch` a specific source

For everything else — a particular Slack channel, a Drive folder, a Jira project, a Gmail label.

### Guided form (recommended)

Type `/watch` with **no arguments** to open the registration form:

1. **Pick a template** from the list (each shows a one-line description).
2. **Fill the fields** — the form shows exactly the parameters that template
   takes, with types, defaults pre-filled, and required fields marked `*`. You
   always choose a **Feed ID** (your handle for this instance). The cadence sits
   under an "advanced" line, pre-filled with the template's default — leave it
   alone unless you want to override it. Fields backed by the provider (a Slack
   channel, a Jira project, a Confluence space) show `↵ choose…`: press Enter to
   pick from a live list instead of typing an id. If the list can't be fetched
   (offline, missing scope), just type the value.
3. Press **Enter** to register. Validation errors keep the form open so you can
   fix the offending field; `Esc` cancels.

Because each value goes in its own field, things that are awkward to type on one
line just work — e.g. a **filesystem path containing spaces** needs no quoting.

### One-line form (power users / scripts)

```
/watch <template> <feed_id> [param=value ...]
```

- `<template>` is `<provider>/<template-name>`, e.g. `slack/channel-archive`.
- `<feed_id>` is your handle for the instance — the channel slug, project key, folder name.
- `[param=value]` is template-specific (the channel ID, the Jira project key, etc.). `/watch list <template>` shows what each template needs. Quote values with spaces: `root="/Users/me/My Notes"`.

Examples:

```
/watch slack/channel-archive design channel=C0123ABCD   # mirrors #design
/watch jira/project-tracker ENG project=ENG             # mirrors the ENG project
/watch drive/folder-sync Reports folder=Reports/2026    # mirrors a Drive folder by name
/watch gmail/sender-filter alerts sender_pattern=alerts@example.com
```

For the full list of templates and their parameters, see [feed templates reference](../reference/feed-templates.md).

> **For template authors:** the form is generated from each template's
> `FeedTemplate::param_schema()` — a required trait method. A new template won't
> compile until it declares its parameters there, so the modal stays in sync
> automatically.

## Discovering what's available

```
/watch list <template>
```

Lists what the provider exposes for that template — channels for Slack, projects for Jira, folders for Drive, etc.

## Backfill from a date

Pass `since=<rfc3339>` to backfill historical data before the cron schedule starts:

```
/watch slack/channel-archive design since=2026-01-01T00:00:00Z
```

The backfill loop walks pagination from `since` forward, persisting the cursor after each page. Server restarts resume from where it left off. If the backfill exhausts a 5-minute rate-limit budget it pauses and the next cron tick continues. A 6-month Gmail backfill is one command.

## Inspecting your feeds

```
/feeds                       # all running feeds
/feeds pause <feed_id>       # pause a feed (cron stops; data stays)
/feeds resume <feed_id>      # resume a paused feed
/feeds run <feed_id>         # trigger an immediate run
/feeds rm <feed_id>          # remove the feed (asks for confirmation)
/feeds rm <feed_id> --yes    # remove without prompt
```

> **Note:** the command is `/feeds rm`, not `/unwatch` (older docs may have used the legacy name).

## Where the data lands

```
~/.arawn/data/<provider>/<template>/<feed_id>/
  ├── meta.json                  # runtime-managed cursor + last status
  └── <template-specific files>  # JSONL logs, JSON snapshots, mirrored bodies
```

`meta.json` carries the cursor, `last_run_at`, `last_status`, `run_count`. The rest of the directory shape is template-specific — see [feed templates reference](../reference/feed-templates.md).

## What's next

- Bind a feed to a workstream so its data becomes a typed knowledge graph: [bind a workstream to a feed](./bind-a-workstream-to-a-feed.md).
- Read feed data with the agent: [read feeds with the agent](./read-feeds-with-the-agent.md).
- Full feed reference (mechanics, cadences, disk-usage estimates): [feeds overview](../reference/feeds-overview.md).
