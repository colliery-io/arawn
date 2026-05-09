---
id: fix-watch-list-slack-channel
level: task
title: "Fix `/watch list slack/channel-archive` mistagging public channels as private"
short_code: "ARAWN-T-0225"
created_at: 2026-05-09T00:00:00+00:00
updated_at: 2026-05-09T00:00:00+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

- [ ] `/watch list slack/channel-archive` shows public channels with no private tag, private channels with `private`, group DMs/Mpim with `dm/group`.
- [ ] Add a unit test covering the three cases by constructing fake `SlackConversationInfo` values (or by mocking at the trait layer if the conversion is split out).
- [ ] Existing channel-archive integration tests still pass.

## Status Updates

*To be added during implementation*
