# Bind a workstream to a feed

*How-to. Attach a feed (or a whole repo / org) to a workstream so the extractor turns each new item into typed entities you can query with `signal_search`.*

A workstream by itself is just a label. A feed by itself is just a local mirror. Binding the two tells arawn: *"watch this source, and use it to populate this workstream's knowledge graph."*

## Prerequisites

- A workstream exists (`/workstream create work`).
- A feed exists (auto-created via `/connect` or explicit `/watch` — see [create a feed](./create-a-feed.md)). Or, for GitHub, the GitHub App is installed (see [connect GitHub](./connect-github.md)).

## How to bind: ask the agent

Workstream binding is an agent-tool operation (`workstream_bind`), not a slash subcommand. The TUI dispatcher only accepts `/workstream {create | list | switch}`; everything else goes through the agent.

Just ask, e.g.:

```
bind the work workstream to gmail-inbox-me
```

The agent calls `workstream_bind { workstream: "work", uri: "gmail-inbox-me" }`. Extraction starts on the next feed run; to force it now, ask the agent to run the feed (or use `/feeds run <feed_id>` directly).

## Direct bind by feed_id

Three example tool calls (these are what the agent runs under the hood — useful for scripting via WS-RPC):

```jsonc
workstream_bind { "workstream": "work", "uri": "gmail-inbox-me" }
workstream_bind { "workstream": "work", "uri": "slack-design" }
workstream_bind { "workstream": "work", "uri": "jira-ENG" }
```

## GitHub URI binds

GitHub has two extra URI schemes that handle the feed registration for you.

### `github:repo:owner/name`

Binds a single repository. arawn registers a `github/repo-mirror` feed for that repo and ties it to the workstream:

```jsonc
workstream_bind { "workstream": "work", "uri": "github:repo:acme/platform-api" }
```

### `github:org:owner`

Binds a whole organization. arawn calls `list_org_repos`, registers one `github/repo-mirror` feed per repo, and ties them all to the workstream. Org binds **supersede** any per-repo binds in the same workstream — if you already had `github:repo:acme/platform-api` bound and then add `github:org:acme`, the per-repo bind is dropped in favor of the org-wide one.

```jsonc
workstream_bind { "workstream": "work", "uri": "github:org:acme" }
```

Or, in chat: *"bind work to the acme org on GitHub."*

> **Note:** all four GitHub templates (`github/notifications`, `github/issues-and-prs`, `github/review-queue`, `github/repo-mirror`) are read-only. v1 of the GitHub integration doesn't write back.

## Hot-register

Bind operations are live — the feed registration happens immediately (no server restart needed). The new feeds appear in `/feeds` right away and start their cron schedule on the next tick. For an immediate first run, ask the agent (or call `/feeds run <feed_id>` directly).

## Unbinding

Ask the agent (*"unbind the github:org:acme binding from work"*), which calls:

```jsonc
workstream_unbind { "workstream": "work", "uri": "github:org:acme" }
```

Removes the binding. The feed itself keeps running (you can still query it directly); the workstream just stops absorbing its rows. For `github:org:` binds, unbinding removes all per-repo feeds the org-expansion created.

## Verifying

Ask the agent to show the workstream, or call the tool directly:

```jsonc
workstream_show { "name": "work" }
```

Lists the bindings on the workstream. After a successful bind you should see your feed_id or the GitHub URI listed.

After extraction has run at least once, the palace will have entities:

```
signal_query { entity_type: "decision", limit: 5 }
```

If this returns rows, extraction is working.

## What's next

- Query the resulting palace: [read feeds with the agent](./read-feeds-with-the-agent.md), or the [first workstream tutorial](../tutorials/first-workstream.md).
- Curate steward proposals: [curate a workstream](./curate-a-workstream.md).
- Understand why workstreams and feeds are separate concepts: [explanation: workstreams](../explanation/workstreams.md).
