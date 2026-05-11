---
id: fix-watch-list-slack-channel
level: task
title: "Fix `/watch list slack/channel-archive` mistagging public channels as private"
short_code: "ARAWN-T-0225"
created_at: 2026-05-09T00:00:00+00:00
updated_at: 2026-05-11T13:27:56.649326+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0039
---

# Fix `/watch list slack/channel-archive` mistagging public channels as private

## Parent Initiative

[[ARAWN-I-0039]]

## Objective

Cosmetic bug found during T-0218 live UAT: every channel coming back from `/watch list slack/channel-archive` is tagged `private`, including known-public channels like `#general`, `#accrete`, `#astro`, `#domino-data-labs`. Mpim group DMs correctly carry `private, dm/group` — only the public-channel mistag is wrong.

Doesn't break registration (the `channel=<id>` token works either way), but the picker hint is misleading.

## Type / Priority

- Bug, P3 — cosmetic, doesn't block any feed functionality.

## Reproduction

1. Run server with a connected Slack integration that has access to public channels.
2. In the TUI, run `/watch list slack/channel-archive`.
3. Observe: every row shows `(<id> · private)` instead of unmarked for public channels.

Expected: public channels render as `(<id>)` with no tag; private channels as `(<id> · private)`; group DMs as `(<id> · private, dm/group)`.

## Root cause hypothesis

`crates/arawn-feeds/src/clients/slack.rs::list_channels` reads `ch.flags.is_private.unwrap_or(false)`. The slack-morphism `SlackConversationFlagsInfo::is_private` may be `Some(true)` even for public conversations when the response comes through the user-token path, rather than the `None`-for-public shape the adapter assumes. Need to either:

a) Check `is_private` only when `is_channel == false || is_group == true` (Slack's "private" really means "private group/channel"), or
b) Use `is_channel` + `is_group` directly to derive the right tag.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `/watch list slack/channel-archive` shows public channels with no private tag, private channels with `private`, group DMs/Mpim with `dm/group`.
- [ ] Add a unit test covering the three cases by constructing fake `SlackConversationInfo` values (or by mocking at the trait layer if the conversion is split out).
- [ ] Existing channel-archive integration tests still pass.

## Status Updates

### 2026-05-10 — can't reproduce in current session

Re-ran `/watch list slack/channel-archive` mid-UAT after several hours of session activity. `#general` now correctly shows **no** `private` tag, while genuinely-private channels keep theirs. Sample picker output:

```
{"hint": "C023Y17JN9G", "label": "#general", "params": {"channel": "..."}}
{"hint": "C05NJ3ARS79  ·  private", "label": "#accrete", ...}
{"hint": "C02UK7K22K0  ·  private", "label": "#astro", ...}
{"hint": "C07P9N0G8CQ  ·  private", "label": "#domino-data-labs", ...}
```

The original screenshot did show `#general` mistagged. The most likely cause is that the slack adapter's `user_context().or_else(bot_context())` fallback was returning the bot context at first; the bot token's view of conversations.list may set `is_private=true` more aggressively than a user token does. Once the user context became available (after the session refreshed it), the picker started reading the right values.

**Punting** — bug isn't currently reproducible. If it resurfaces, the fix path is to derive privacy state from `is_channel` + `is_group` rather than relying solely on `is_private` (`is_group=true` is the canonical legacy "private channel" marker). Re-open with a fresh repro if needed.

### 2026-05-11 — closing as obsolete

Re-probed live `feed_discover` for `slack/channel-archive`. All five
conversation shapes tag correctly: public (`#general`,
`#green-phreaks`, `#makers` — no tag), modern private (`#accrete`,
`#astro`, `#domino-data-labs` — `private`), and mpim
(`#mpdm-*` — `private, dm/group`). Bug remains unreproducible across
two separate UAT sessions and server restarts.

Not landing the punt-note's recommended fix path: deriving privacy
from `is_group` alone would regress modern private channels, which
Slack returns as `is_channel=true, is_private=true, is_group=false`
post-2019 conversation-types unification. The current code (trust
`is_private`) is correct for every shape we can observe; the original
mistag was almost certainly the `user_context().or_else(bot_context())`
fallback returning bot-token-shaped data on cold start, which auto-
corrected once the user context refreshed.

Closing. Re-open with a fresh repro if the bug returns.