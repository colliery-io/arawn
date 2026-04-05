---
id: integration-test-suite-end-to-end
level: initiative
title: "Integration test suite — end-to-end testing across subsystems"
short_code: "ARAWN-I-0013"
created_at: 2026-04-04T16:29:26.521762+00:00
updated_at: 2026-04-05T17:51:20.070654+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: integration-test-suite-end-to-end
---

# Integration test suite — end-to-end testing across subsystems Initiative

## Context

We have ~640 unit tests but zero integration tests that exercise subsystems together. We've built hooks, skills, plugins (with marketplace + installer + cache), permissions, MCP client, config hot-reload, and a query engine — all tested in isolation. The gaps:

- No test that installs a plugin from a marketplace and verifies its components load
- No test that configures a PreToolUse hook and verifies it blocks a tool call through the engine
- No test that loads a skill and invokes it via SkillTool through the engine
- No test that modifies arawn.toml and verifies the ConfigWatcher picks up changes
- No test that exercises the full query engine loop with tools + permissions + hooks together
- No test for the `arawn plugin` CLI commands against a real cache directory

The existing `crates/arawn-tests/` crate has integration tests for compaction, engine persistence, local service, plugin loading, and websocket — but nothing for the new subsystems.

## Goals & Non-Goals

**Goals:**
- Integration tests for plugin install → load → components available
- Integration tests for hooks end-to-end (PreToolUse blocks, PostToolUse fires)
- Integration tests for skill loading + SkillTool invocation
- Integration tests for config hot-reload (permissions, MCP server diff)
- Integration tests for the full engine loop with permissions + hooks wired
- Test fixtures: temp directories with realistic plugin/marketplace/config structures

**Non-Goals:**
- UI/TUI testing (snapshot tests already cover that)
- Load/stress testing
- Mocking LLM responses (existing engine tests already do that)

## Detailed Design (Revised 2026-04-05)

### Key Discovery: Codebase Is More Testable Than Assumed

Deep review revealed the codebase was designed well for testing:
- `discover_plugins(plugins_root: &Path)` — path-parameterized, testable with temp dirs (no trait refactor needed)
- `load_plugin_components(&LoadedPlugin)` — takes struct, not filesystem globals
- `QueryEngine` already has `with_permission_checker()`, `with_hook_runner()`, `with_skill_registry()`, `with_plugin_registry()`
- `PermissionChecker::new(rules).with_mode().with_prompter()` — composable, `MockModalPrompt` exists
- `HookRunner::new(config, cwd)` — data-driven config, testable with shell commands

### The Actual Gap

`TestHarnessBuilder` (in `testing.rs`) only exposes tools + LLM script. It doesn't wire permissions, hooks, skills, or plugins into the engine. Extending the harness is the one refactor that unlocks everything.

### Test categories

1. **Permission + engine** — deny/allow rules, BypassPermissions mode, AcceptEdits mode, ask→MockModalPrompt, session grants across turns
2. **Hooks + engine** — PreToolUse blocking/allowing hooks, PostToolUse firing, content pattern matching, multi-hook aggregation
3. **Skill + engine** — in-memory registration + invocation, markdown file parsing + loading, skill-not-found error, user_invocable filtering
4. **Plugin discovery + components** — temp dir with plugin.json + component files, discover_plugins, load_plugin_components, register into engine registries, disabled/invalid plugin handling
5. **Full pipeline** — multi-turn conversation with permissions + hooks + skills + tools all wired, exercises all code paths
6. **Hot-reload API** — update_rules/update_mode mid-session, verify behavior changes on next tool call

### What to defer

- **MCP server integration** — requires mock MCP server binary or rmcp-level mock. Lower priority.
- **ConfigWatcher filesystem watching** — timing-dependent. The hot-reload API is testable now; the watcher can come later.

### Test infrastructure

All tests use `tempfile::TempDir` for isolation. No global state, no real LLM calls, no network. Tests go in `crates/arawn-tests/tests/` alongside existing integration tests.

### Task dependency graph

```
T-foundation (harness) ──┬──> T-permissions
                         ├──> T-hooks
                         ├──> T-skills
                         └──> T-hot-reload

(independent)            ───> T-plugins  (tests loader APIs directly, no harness needed)

T-permissions + T-hooks + T-skills + T-plugins ──> T-full-pipeline
```

## Alternatives Considered

- **Trait-based plugin provider refactor**: Considered but unnecessary — plugin APIs are already path-parameterized
- **Mock MCP server**: Deferred — would require building/maintaining a test binary
- **Property-based testing**: Overkill for integration tests — save for fuzz testing later
- **Docker-based test harness**: Too heavy for CI. Temp dirs are sufficient.
- **Separate test binary per subsystem**: Unnecessary — `arawn-tests` crate already aggregates integration tests