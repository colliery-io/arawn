---
id: plugin-cli-commands-arawn-plugin
level: task
title: "Plugin CLI commands — arawn plugin install/uninstall/enable/disable/list/marketplace"
short_code: "ARAWN-T-0091"
created_at: 2026-04-04T13:05:32.278210+00:00
updated_at: 2026-04-04T14:14:56.115410+00:00
parent: ARAWN-I-0010
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0010
---

# Plugin CLI commands — arawn plugin install/uninstall/enable/disable/list/marketplace

## Objective

Add `arawn plugin` CLI subcommands so users can manage plugins from the command line.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `arawn plugin install <name@marketplace>` — install a plugin (calls installer from T-0090)
- [ ] `arawn plugin uninstall <name@marketplace>` — uninstall a plugin
- [ ] `arawn plugin enable <name@marketplace>` — enable a disabled plugin
- [ ] `arawn plugin disable <name@marketplace>` — disable without uninstalling
- [ ] `arawn plugin list` — list installed plugins with status (enabled/disabled, version, source)
- [ ] `arawn plugin marketplace add <github-repo-or-url>` — add a marketplace source
- [ ] `arawn plugin marketplace list` — list registered marketplaces and their plugins
- [ ] `--plugin-dir <path>` flag on main `arawn` command — load plugin from directory for session only
- [ ] All commands print clear success/error messages
- [ ] Plugin list shows `name@marketplace` format, version, enabled status, component summary (agents/skills/hooks)

## Implementation Notes

- Add subcommands to `crates/arawn/src/commands/` using clap
- Depends on T-0088 (identifiers), T-0089 (marketplace), T-0090 (installer)
- `--plugin-dir` is a top-level flag, not a plugin subcommand — wire into main arg parser
- `plugin list` should work even when the engine isn't running (just reads cache + settings)
- Consider adding `arawn plugin validate <path>` for plugin authors to check their manifest

## Status Updates

*To be added during implementation*