---
id: discovery-pickers-watch-list
level: task
title: "Discovery pickers — `/watch list <template>` (T-0219 slice 3)"
short_code: "ARAWN-T-0224"
created_at: 2026-05-09T00:00:00+00:00
updated_at: 2026-05-09T00:00:00+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: true
initiative_id: ARAWN-I-0039
---

# Discovery pickers — `/watch list <template>`

## Parent Initiative

[[ARAWN-I-0039]]

## Objective

Closes the slice-3 deferral from T-0219. Discovery pickers help users register feeds without copying provider IDs from external tooling — `/watch list slack/channel-archive` shows every channel + its `channel=C0123ABC` token.

## Status — landed

Picker landed as a chat-pane numbered list rather than a ratatui modal. Modal upgrade is optional sugar — the text form is fully functional and discoverable.

### Trait additions

- `FeedTemplate::discover(&ctx) -> Result<Option<Vec<DiscoveryRow>>, FeedError>` — default `Ok(None)` for free-form templates. The three pickable templates override:
  - `slack/channel-archive` → `slack.list_channels()`
  - `jira/project-tracker` → `atlassian.list_jira_projects()`
  - `confluence/space-archive` → `atlassian.list_confluence_spaces()`
- `DiscoveryRow { label, hint, params }`. The picker shows `label`, hands `params` straight to `feed_register` on selection.

### Provider extensions

- `SlackFeedClient::list_channels()` returns `Vec<SlackChannel>` (id, name, is_private, is_dm) — public + private + Mpim, excludes archived.
- `AtlassianFeedClient::list_jira_projects()` and `list_confluence_spaces()` round-trip through the v3 OpenAPI client and Confluence v2 `/spaces`.

### Runtime + service

- `FeedRuntime::discover_template(template) -> Option<Vec<DiscoveryRow>>` — looks up the template, builds a `TemplateCtx` from the runtime's clients, calls `template.discover()`.
- `ArawnService::feed_discover(template) -> FeedDiscoverDto` exposing `picker_supported`, `rows`, and the original template name. RPC method `feed_discover` registered.

### TUI

- `/watch list` (no template) → static help listing the 12 templates and a pointer at the picker shortcut.
- `/watch list <template>` → calls `feed_discover`. Renders rows as a numbered list with a copy-pasteable `key=value` token per row. The user copies the token, then runs `/watch <template> <feed_id> key=value`.
- Free-form templates show "doesn't support discovery — use the typed form" with an example.
- Empty rows from a connected provider show "no values" rather than an empty list.

### Why text-mode rather than a real modal

A ratatui modal with type-to-filter + keybinds would be ~150–250 LOC of new UI code (focus management, key routing, layered render). The chat-pane numbered list:

- Works today, no modal infra to maintain.
- Lets the user keep their cursor in the input buffer (no focus break).
- Doesn't need a stateful pick-then-submit flow — the user keeps control of the registration call.

A modal-shaped picker is a natural follow-up if the chat-pane form turns out to be ergonomically lacking.

### Tests

- `tests/discovery.rs` (5 integration tests):
  - `slack_channel_archive_discovers_channels` — sorting, private/DM hint markers, `channel=...` params.
  - `jira_project_tracker_discovers_projects` — sorting, label shape, `project=KEY` params.
  - `confluence_space_archive_discovers_spaces` — empty-name fallback.
  - `discover_returns_none_when_provider_missing` — short-circuits to `None` when the integration isn't connected so the TUI can render a "not connected" message instead of an empty modal.
  - `non_pickable_template_returns_none` — sender-filter etc. default to `None`.
- `command::tests::watch_list_dispatches_to_feed_discover` — parser dispatches both forms.
- `command::tests::watch_list_doesnt_swallow_a_template_named_listed` — defensive parser test.

139 arawn-feeds + 150 arawn-tui tests green. `angreal check workspace` and `angreal check clippy` clean.

T-0219 is now fully closed across all four originally-planned slices. I-0039 remains complete pending only T-0218 (UAT + agent read-pattern docs).
