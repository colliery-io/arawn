---
id: hook-config-loading-parse-hooks
level: task
title: "Hook config loading — parse hooks from settings.json with user/project priority merging"
short_code: "ARAWN-T-0080"
created_at: 2026-04-04T02:16:25.225356+00:00
updated_at: 2026-04-04T02:32:51.720738+00:00
parent: ARAWN-I-0008
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0008
---

# Hook config loading — parse hooks from settings.json with user/project priority merging

## Objective

Load hook configuration from settings.json files (user-level `~/.arawn/settings.json` and project-level `.arawn/settings.json`), deserialize into `HookConfig`, and merge with user settings taking priority over project settings.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Parse `hooks` key from settings.json into `HookConfig` struct
- [ ] Load from user settings (`~/.arawn/settings.json`) and project settings (`.arawn/settings.json`)
- [ ] User settings take priority — if same event+matcher defined in both, user wins
- [ ] Hooks from both sources are merged (not replaced) — user hooks run alongside project hooks
- [ ] Deduplication: identical command+matcher across sources collapses to one hook
- [ ] Graceful handling of missing files, missing `hooks` key, or malformed entries (warn and skip)
- [ ] Integrates with existing settings infrastructure (if any) or establishes the pattern
- [ ] Unit tests: load from single source, merge two sources, dedup, missing file, malformed JSON

## Implementation Notes

- Depends on T-0078 for `HookConfig` and related types
- Check how the permission system loads settings — follow the same pattern
- Claude Code deduplicates by command + matcher + if-condition as key
- Consider whether we need a snapshot mechanism (Claude Code captures config at startup) — can defer to future work

## Status Updates

*To be added during implementation*