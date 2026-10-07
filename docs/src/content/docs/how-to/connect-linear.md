---
title: "Connect Linear"
---
*How-to. ~3 minutes. Linear takes a personal API key: no OAuth app, no browser flow.*

By the end of this guide arawn holds one Linear personal API key, encrypted at rest. The agent can list and read your issues, comment on them, and change their state, assignee and priority. The `linear-mine` feed brings the issues that are assigned to you or created by you into arawn, and the urgent or due-soon ones reach the morning brief.

One key covers one user in one workspace. To use a second workspace, replace the key (step 2).

## Prerequisites

- arawn installed. The server can be running or not.
- A Linear account that may create personal API keys. Admins can always create them; members only when the workspace allows it (**Settings → Administration → API → Member API keys**).

## 1. Create a personal API key

<!-- VERIFY: 2026-10-07 — linear.app Settings > Account > Security & access; key permission options. -->

In Linear, go to **Settings → Account → Security & access** (<https://linear.app/settings/account/security>). Under **Personal API keys**, create a key:

- **Name:** `arawn`.
- **Permissions:** full access, or at least **Read** and **Write**. Read is for the feed and the read tools; Write is for the comment, state, assignee and priority tools. With Read only, the write tools fail with a provider error.
- **Team access:** the teams arawn may see. Issues of other teams are not in the feed, and the tools cannot change them.

Copy the key. It starts with `lin_api_`.

## 2. Give the key to arawn

```sh
arawn setup linear
```

Paste the key at the hidden prompt. arawn checks it with Linear's `viewer` query, then saves it encrypted in `<data_dir>/integrations/linear/linear.bin`, with the same key file and cipher as the OAuth tokens. Nothing is written to `arawn.toml`. A key that Linear rejects is not saved.

For a script, export the key for this one command:

```sh
ARAWN_LINEAR_API_KEY=lin_api_… arawn setup linear
```

`ARAWN_LINEAR_API_KEY` is read only by `arawn setup linear`. The server never reads it: the encrypted file is the only source of the key.

To replace the key (a new key, or another workspace), run `arawn setup linear` again. It asks before it replaces the saved key.

There is no `arawn connect linear` step: `arawn connect linear` and `/connect linear` only point at `arawn setup linear`. A running server uses the new key from its next call, with no restart.

## 3. Check it

```sh
arawn doctor
```

`integration:linear` is PASS. `/status` in the TUI shows `linear` as connected.

## 4. The feed

`linear/my-issues` pulls the issues assigned to you or created by you every 30 minutes: state, priority, due date, team, labels, assignee, creator and the ten newest comments. It feeds the `linear_issues` projection, which `feed_search` reads.

The server creates the feed as `linear-mine` when it starts with a saved key. If you saved the key while the server was running, restart it, or create the feed now:

```
/watch linear/my-issues linear-mine
```

`arawn init --profile work` declares `linear-mine` in `arawn.toml` and binds it to the `work` lens.

The morning brief lists your open issues (state not completed or canceled) that are assigned to you and are urgent or high priority, or due within seven days (overdue included).

## Revoke

- To stop arawn using the key: `arawn disconnect linear`. It deletes the saved key; the feed then fails with "reconnect needed" until you run `arawn setup linear` again.
- To revoke the key itself: delete it under **Settings → Account → Security & access** in Linear. arawn then reports an authentication error, and `arawn setup linear` with a new key fixes it.

## Rate limits

Linear allows each API key 2,500 requests and 3,000,000 complexity points per hour. When a limit is spent, arawn waits until the reset time that Linear gives (`X-RateLimit-Requests-Reset`): a feed run reports `rate limited` and the next run continues from its cursor; a tool call gives the error with the wait. The feed uses one request per 50 issues per run.

## Troubleshooting

| Symptom | Fix |
|---|---|
| `Linear did not accept the key` during setup | The key is wrong, revoked, or was copied with spaces. Create a new key and run `arawn setup linear` again. |
| `integration:linear` is `token_error` | The saved file cannot be decrypted (for example, `tokens/.master.key` changed). Run `arawn setup linear` again. |
| A write tool fails with a provider error | The key has no Write permission, or no access to the issue's team. Give it the permission in Linear, or create a new key. |
| An issue is missing from the feed | The key has no access to its team, or the issue is neither assigned to you nor created by you. |

## See also

- [Integrations reference](../../reference/integrations/#linear)
- [Agent tools reference](../../reference/agent-tools/#linear-6)
- [Feed templates reference](../../reference/feed-templates/#linear)
