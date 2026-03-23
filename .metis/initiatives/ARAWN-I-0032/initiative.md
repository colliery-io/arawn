---
id: functional-completion-polish
level: initiative
title: "Functional Completion & Polish"
short_code: "ARAWN-I-0032"
created_at: 2026-03-22T00:39:12.579570+00:00
updated_at: 2026-03-22T22:24:34.732035+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: functional-completion-polish
---

# Functional Completion & Polish Initiative

## Context

Functional readiness assessment (March 2026) scored Arawn 7/10 for daily use. The core agent loop works end-to-end, but several features are implemented but not wired up, and several quality-of-life improvements are needed for comfortable daily use.

## Goals & Non-Goals

**Goals:**
- Wire up all implemented-but-unregistered features (WebSearchTool, plugin prompts)
- Implement daemon mode for background operation
- Add first-run experience and configuration documentation
- Fix remaining test failures (8 log tests, 3 sandbox tests)
- Add session persistence across server restarts

**Non-Goals:**
- New major features (new tools, new UIs)
- Mobile or web client
- Multi-user features

## Detailed Design

### Critical Gaps (P0)
1. **Register WebSearchTool** (`arawn/src/commands/start.rs:~385`): Add `WebSearchTool` to the tool registry. The tool is fully implemented with Brave/Serper/Tavily/DuckDuckGo support but never registered. 1-line fix.
2. **Wire plugin prompt fragments** (`arawn/src/commands/start.rs:~652`): Populate `plugin_prompts` from loaded plugins' prompt fragments so plugin instructions flow into the system prompt.
3. **Wire plugin CLI tools**: Comment at line 789 of start.rs notes "CLI tools (commands/) and prompt fragments are not yet implemented." Wire plugin-defined commands into the CLI.

### Daemon Mode (P1)
4. **Implement `--daemon` flag** (`arawn/src/commands/start.rs:105-108`): Use `daemonize` crate or fork-and-exec pattern to run server in background. Write PID file for management.
5. **`arawn stop` command**: Send SIGTERM to the daemon PID file.

### Configuration & Onboarding (P1)
6. **Example config file**: Create `arawn.example.toml` with all sections documented with inline comments.
7. **`arawn init` guided setup**: Interactive first-run wizard that asks for LLM provider, API key, and writes initial config.
8. **Config validation on startup**: Fail fast with clear messages for invalid config values.

### Test Fixes (P1)
9. **Fix 8 log integration test failures**: Likely temp dir setup issues in `arawn/tests/command_integration.rs`.
10. **Fix 3 sandbox test failures**: Platform detection in `arawn-sandbox/src/manager.rs` for macOS 15.x.

### Session Persistence (P2)
11. **Session cache save/load**: Add persistence to `arawn-server/src/session_cache.rs` — save dirty sessions on eviction and shutdown, reload on startup.
12. **Versioning strategy**: Move from `0.0.0` to semantic versioning with changelog.

## Implementation Plan

- Phase 1: P0 wiring fixes (1 day)
- Phase 2: P1 daemon mode + config + test fixes (3-4 days)
- Phase 3: P2 session persistence + versioning (2-3 days)