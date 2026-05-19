# Template Catalog

Twelve templates ship today, across six providers. This page is the
contract the agent reads — what params each takes, what cadence it
runs on, and what lands on disk.

> Paths below are shown relative to `~/.arawn/data/`. So
> `slack/channel-archive/design/...` means
> `~/.arawn/data/slack/channel-archive/design/...`.

## Slack

### `slack/channel-archive`

Append every message in one Slack channel to JSONL, time-partitioned
by day, plus per-thread reply files.

| Field | Value |
|---|---|
| Param | `channel: string` (name like `#design` or id like `C123ABC`) |
| Default cadence | `*/15 * * * *` |
| Auto-create | No — use `/watch slack/channel-archive <channel>` |

```text
slack/channel-archive/<feed_id>/
  ├── meta.json                       # cursor: { latest_ts, threads }
  ├── 2026-05-08.jsonl                # parents + standalone msgs, by ts
  ├── 2026-05-07.jsonl
  └── threads/
      ├── 1746700000.000100.jsonl     # parent + replies for one thread
      └── ...
```

Channel and thread cursors advance independently — a single bad
thread doesn't block the channel or other threads.

### `slack/my-mentions`

Every message anywhere in the workspace containing an `@me` mention.

| Field | Value |
|---|---|
| Param | (none) |
| Default cadence | `*/15 * * * *` |
| Auto-create | Yes — singleton, on `/connect slack` |

```text
slack/my-mentions/me/
  ├── meta.json                       # cursor: { my_user_id, latest_ts }
  ├── 2026-05-08.jsonl                # mention messages by their Slack ts
  └── 2026-05-07.jsonl
```

No threads — a mention is one moment. If you want the surrounding
discussion, archive the parent channel.

### `slack/dm-archive`

Mirror a 1-on-1 DM conversation, same dual-layer storage as
channel-archive.

| Field | Value |
|---|---|
| Param | `user: string` (Slack user id `UABC123` or username) |
| Default cadence | `0 * * * *` (hourly) |
| Auto-create | No |

```text
slack/dm-archive/<feed_id>/
  ├── meta.json
  ├── 2026-05-08.jsonl                # top-level DM messages
  └── threads/
      └── <parent_ts>.jsonl
```

## Gmail

All three Gmail templates write the same shape:

```text
<feed_dir>/
  ├── meta.json                  # cursor: { latest_internal_date }
  ├── 2026-05-08/
  │   ├── <msg_id_a>.json        # full Gmail Message JSON
  │   └── <msg_id_b>.json
  └── 2026-05-07/
      └── <msg_id_c>.json
```

Files are partitioned by Gmail's `internalDate` (canonical send time),
not fetch time. Re-runs are cheap — if `<day>/<msg_id>.json` already
exists, the `messages.get` API call is skipped entirely.

### `gmail/inbox-archive`

| Field | Value |
|---|---|
| Param | `days_back: u32` (default 7) |
| Default cadence | `*/15 * * * *` |
| Auto-create | Yes — singleton "me" on `/connect gmail` |

### `gmail/label-archive`

| Field | Value |
|---|---|
| Required | `label: string` (built-in like `IMPORTANT` or user label; nested with `/`) |
| Optional | `days_back: u32` (default 30) |
| Default cadence | `*/30 * * * *` |
| Auto-create | No |

### `gmail/sender-filter`

| Field | Value |
|---|---|
| Required | `sender_pattern: string` (any value Gmail's `from:` operator accepts) |
| Optional | `days_back: u32` (default 14) |
| Default cadence | `*/30 * * * *` |
| Auto-create | No |

## Calendar

### `calendar/upcoming-archive`

Rolling snapshot of every event between now and `window_days` ahead.

| Field | Value |
|---|---|
| Optional | `calendar: string` (default `primary`), `window_days: u32` (default 7) |
| Default cadence | `*/30 * * * *` |
| Auto-create | Yes — singleton, on `/connect google_calendar` |

```text
calendar/upcoming-archive/<feed_id>/
  ├── meta.json                       # cursor: { last_synced_at }
  └── events/
      ├── <event_id>.json             # current state, overwrite-on-update
      └── ...
```

One file per `event_id`, **overwritten on update**. Calendar events
are mutable — the agent reads "what's on my calendar now," not "what
was there two hours ago."

## Drive

### `drive/recent`

Every Drive file modified in the last N days. Metadata only — bodies
aren't mirrored, but the agent can call `drive_read` if it needs one.

| Field | Value |
|---|---|
| Optional | `days_back: u32` (default 7, validated 1..=90) |
| Default cadence | `*/30 * * * *` |
| Auto-create | Yes — singleton "me" on `/connect google_drive` |

```text
drive/recent/<feed_id>/
  ├── meta.json                       # cursor: { latest_modified_iso }
  ├── 2026-05-08/
  │   ├── <file_id_a>.json            # DriveFile metadata snapshot
  │   └── <file_id_b>.json
  └── 2026-05-07/
      └── <file_id_c>.json
```

### `drive/folder-sync`

Rsync-style mirror of a Drive folder onto local disk. Bodies are
downloaded; renames and moves are handled (old path deleted, new
written in the same run).

| Field | Value |
|---|---|
| Required | `folder: string` (folder id, `"root"`, or a path like `"Reports/2026"`) |
| Default cadence | `0 * * * *` (hourly) |
| Auto-create | No |

```text
drive/folder-sync/<feed_id>/
  ├── meta.json             # cursor: { files: { <id>: { token, path } } }
  ├── <subfolder>/
  │   └── <file>            # native bytes
  └── <file>
```

Google native types (Docs/Sheets/Slides/Drawings) are exported to
markdown/csv/txt/png with a matching extension. Unsupported native
types (forms, sites, scripts) are skipped with a warn-level log.

## Jira

### `jira/project-tracker`

Issues + comments + history for a Jira project. Snapshot per issue is
overwritten; comments and history are append-only logs.

| Field | Value |
|---|---|
| Required | `project: string` (key like `"ENG"`) |
| Default cadence | `*/30 * * * *` |
| Auto-create | No |

```text
jira/project-tracker/<feed_id>/
  ├── meta.json                       # cursor (see below)
  └── <ISSUE-KEY>/
      ├── issue.json                  # latest snapshot, overwrite
      ├── comments.jsonl              # append-only, deduped by id
      └── history.jsonl               # append-only, deduped by id
```

Cursor combines a feed-level `latest_updated_iso` and a per-issue
`{ last_comment_id, last_history_id }` map so each issue's logs
advance independently.

### `jira/assignee-tracker`

Personal feed: every Jira issue currently assigned to you. Lighter
than project-tracker — snapshot only, no logs.

| Field | Value |
|---|---|
| Param | (none — uses `currentUser()` JQL) |
| Default cadence | `*/30 * * * *` |
| Auto-create | Yes — singleton "me" on `/connect atlassian` |

```text
jira/assignee-tracker/me/
  ├── meta.json                       # cursor: { latest_updated_iso }
  └── <ISSUE-KEY>/
      └── issue.json                  # snapshot only, overwrite
```

## Confluence

### `confluence/space-archive`

Every page in a Confluence space: metadata + raw storage-format body.
One directory per page; both files are overwrite-on-update.

| Field | Value |
|---|---|
| Required | `space_key: string` (e.g. `"ENG"`) |
| Default cadence | `*/30 * * * *` |
| Auto-create | No |

```text
confluence/space-archive/<feed_id>/
  ├── meta.json                       # cursor: { last_modified_iso }
  └── <page_id>/
      ├── page.json                   # page metadata + version
      └── body.storage.xml            # raw body, overwrite-on-update
```

Bodies are written verbatim as Confluence storage format (XML). No
ADF or markdown conversion at archive time — agents prefer
source-of-truth markup.

## Quick reference: cadence + auto-create

| Template | Cadence | Auto-create |
|---|---|---|
| `slack/channel-archive` | every 15 min | No |
| `slack/my-mentions` | every 15 min | Yes (singleton) |
| `slack/dm-archive` | hourly | No |
| `gmail/inbox-archive` | every 15 min | Yes (singleton) |
| `gmail/label-archive` | every 30 min | No |
| `gmail/sender-filter` | every 30 min | No |
| `calendar/upcoming-archive` | every 30 min | Yes (singleton) |
| `drive/recent` | every 30 min | Yes (singleton) |
| `drive/folder-sync` | hourly | No |
| `jira/project-tracker` | every 30 min | No |
| `jira/assignee-tracker` | every 30 min | Yes (singleton) |
| `confluence/space-archive` | every 30 min | No |
