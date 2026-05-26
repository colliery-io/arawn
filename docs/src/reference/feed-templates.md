# Feed templates

*Reference. Every shipped feed template with parameters, cadence, and on-disk layout.*

Seventeen templates ship today across eight providers. This page is the contract the agent reads — what params each takes, what cadence it runs on, and what lands on disk.

(An 18th "stub/echo" template is registered as a test fixture and isn't user-facing.)

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
| Optional | `calendar_id: string` (default `primary`), `window_days: u32` (default 7) |
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
| Required | `project: string` (key like `"ENG"` or numeric id) |
| Default cadence | `*/30 * * * *` |
| Auto-create | No |

At registration time arawn calls `resolve_project` against your Jira
instance to verify the key/id exists. A typo (`"EGN"` instead of
`"ENG"`) fails fast with a clear error instead of silently producing
empty runs.

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

## GitHub

GitHub feeds use the [GitHub App](../how-to/connect-github.md) integration (not OAuth). All four templates default to `*/30 * * * *`. v1 is read-only.

### `github/notifications`

Your `/notifications` inbox.

| Field | Value |
|---|---|
| Param | (none) |
| Default cadence | `*/30 * * * *` |
| Auto-create | No (today; tied to per-user install) |

```text
github/notifications/<feed_id>/
  ├── meta.json
  └── notifications.jsonl     # appended, deduped by id
```

### `github/issues-and-prs`

Open + recently-closed issues and PRs you authored or are assigned to.

| Field | Value |
|---|---|
| Param | (none — uses your authenticated user) |
| Default cadence | `*/30 * * * *` |
| Auto-create | No |

```text
github/issues-and-prs/<feed_id>/
  ├── meta.json
  ├── issues.jsonl            # append-only, deduped by id
  └── prs.jsonl
```

### `github/review-queue`

PRs where you're a requested reviewer.

| Field | Value |
|---|---|
| Param | (none) |
| Default cadence | `*/30 * * * *` |
| Auto-create | No |

```text
github/review-queue/<feed_id>/
  ├── meta.json
  └── review_requests.jsonl
```

### `github/repo-mirror`

Full snapshot of a repo's commits, open issues, PRs, and issue/PR comments. Used by the `github:repo:owner/name` and `github:org:owner` workstream binding schemes — bind expands org URIs to one `repo-mirror` per repo.

| Field | Value |
|---|---|
| Required | `owner: string`, `name: string` (two separate params, NOT a combined `owner/name` string) |
| Default cadence | `*/30 * * * *` |
| Auto-create | No (registered by `workstream_bind` with a GitHub URI) |

```text
github/repo-mirror/<feed_id>/<owner>/<name>/
  ├── meta.json
  ├── commits/
  │   └── <sha>.json
  ├── issues/
  │   └── <number>.json
  ├── prs/
  │   └── <number>.json
  └── comments/
      └── <comment_id>.json
```

Per-row JSON files in nested kind directories — not flat JSONL. The on-disk layout is shared by single-repo and org-expanded variants; the org URI just creates N feeds, each with one repo's worth of state.

## Filesystem

### `filesystem/folder`

Watch a local folder and emit a signal whenever a text file is created, modified, or deleted. Built for raw text drops — meeting transcripts, exported notes — **not** for code trees (hence the default excludes). It needs no provider connection; it reads the local disk directly.

| Field | Value |
|---|---|
| Required | `root: string` (absolute path to the folder to watch) |
| Optional | `recursive: bool` (default `true`), `include: [string]` (glob list, default `["**/*"]`), `exclude: [string]` (glob list, default below) |
| Default cadence | `*/15 * * * *` |
| Auto-create | No — use `/watch filesystem/folder root=/path/to/notes` |

Default excludes: `.git/**`, `target/**`, `node_modules/**`, `.venv/**`, `__pycache__/**`, `.DS_Store`.

```text
filesystem/folder/<feed_id>/
  ├── meta.json          # cursor: { files: { <abs_path>: { mtime, size } } }
  └── signals.jsonl      # append-only, one change event per line
```

Each `signals.jsonl` line is one change event:

```json
{ "source": "/Users/me/notes", "path": "/Users/me/notes/standup.md",
  "rel_path": "standup.md", "event": "created",
  "ts": "2026-05-25T10:00:00+00:00", "size_bytes": 812, "mtime": 1748160000 }
```

**How change is detected.** Each scan walks `root`, builds a `(mtime, size)` fingerprint per matching file, and diffs it against the cursor's map from the previous scan. New path → `created`; changed fingerprint → `modified`; gone → `deleted` (with `size_bytes` and `mtime` as `null`). Unchanged files emit nothing. The first scan after registration has an empty cursor, so every matching file is emitted as `created` — an automatic initial index.

**Caveats.**

- **Fingerprint, not content hash.** A pathological in-place edit that preserves *both* mtime and size is not detected. Normal editor saves bump mtime, so this is rare in practice.
- **Renames are delete + create.** A rename surfaces as a `deleted` event for the old path and a `created` event for the new one — there is no rename correlation.
- **Path-depth restriction.** `root` must be an absolute path at least two levels deep (3+ path components). Watching `/`, `/Users`, or another volume/home root is rejected at registration.
- **Glob matching is relative to `root`.** `exclude` always wins over `include`.
- **Polling, not push.** Detection latency is one cadence tick. The 15-minute floor (shared by all feeds) is the fastest available; sub-minute "live" watching is out of scope for this surface.

`include` / `exclude` globs are matched against the path relative to `root`, so `exclude = ["drafts/**"]` skips everything under `<root>/drafts/`.

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
| `github/notifications` | every 30 min | No |
| `github/issues-and-prs` | every 30 min | No |
| `github/review-queue` | every 30 min | No |
| `github/repo-mirror` | every 30 min | No (registered by workstream bind) |
| `filesystem/folder` | every 15 min | No |

## Related

- [Feeds overview reference](./feeds-overview.md) — on-disk layout, status states, backfill.
- [`feed_search` tool reference](./feed-search-tool.md).
- [Create a feed how-to](../how-to/create-a-feed.md).
- [Bind a workstream to a feed how-to](../how-to/bind-a-workstream-to-a-feed.md).
