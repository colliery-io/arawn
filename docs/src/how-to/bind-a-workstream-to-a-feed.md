# Bind a workstream to a feed

*How-to. Attach a feed (or a whole repo / org) to a workstream so the extractor turns each new item into typed entities you can query with `signal_search`.*

A workstream by itself is just a label. A feed by itself is just a local mirror. Binding the two tells arawn: *"watch this source, and use it to populate this workstream's knowledge graph."*

## Prerequisites

- A workstream exists (`/workstream create work`).
- A feed exists (auto-created via `/connect` or explicit `/watch` — see [create a feed](./create-a-feed.md)). Or, for GitHub, the GitHub App is installed (see [connect GitHub](./connect-github.md)).

## Direct bind by feed_id

```
/workstream bind <ws> <feed_id>
```

Examples:

```
/workstream bind work gmail-inbox-me
/workstream bind work slack-design
/workstream bind work jira-ENG
```

Extraction starts on the next feed run. To force it now:

```
/feeds run <feed_id>
```

## GitHub URI binds

GitHub has two extra URI schemes that handle the feed registration for you.

### `github:repo:owner/name`

Binds a single repository. arawn registers a `github/repo-mirror` feed for that repo and ties it to the workstream:

```
/workstream bind work github:repo:acme/platform-api
```

### `github:org:owner`

Binds a whole organization. arawn calls `list_org_repos`, registers one `github/repo-mirror` feed per repo, and ties them all to the workstream. Org binds **supersede** any per-repo binds in the same workstream — if you already had `github:repo:acme/platform-api` bound and then add `github:org:acme`, the per-repo bind is dropped in favor of the org-wide one.

```
/workstream bind work github:org:acme
```

> **Note:** all four GitHub templates (`github/notifications`, `github/issues-and-prs`, `github/review-queue`, `github/repo-mirror`) are read-only. v1 of the GitHub integration doesn't write back.

## Hot-register

Bind operations are live — the feed registration happens immediately (no server restart needed). The new feeds appear in `/feeds` right away and start their cron schedule on the next tick. For an immediate first run, `/feeds run <feed_id>`.

## Unbinding

```
/workstream unbind <ws> <feed_id_or_uri>
```

Removes the binding. The feed itself keeps running (you can still query it directly); the workstream just stops absorbing its rows.

For `github:org:` binds, unbinding removes all per-repo feeds the org-expansion created.

## Verifying

```
/workstream show <ws>
```

Lists the bindings on a workstream. After a successful bind you should see your feed_id or the GitHub URI listed.

After extraction has run at least once, the palace will have entities:

```
signal_query { entity_type: "decision", limit: 5 }
```

If this returns rows, extraction is working.

## What's next

- Query the resulting palace: [read feeds with the agent](./read-feeds-with-the-agent.md), or the [first workstream tutorial](../tutorials/first-workstream.md).
- Curate steward proposals: [curate a workstream](./curate-a-workstream.md).
- Understand why workstreams and feeds are separate concepts: [explanation: workstreams](../explanation/workstreams.md).
