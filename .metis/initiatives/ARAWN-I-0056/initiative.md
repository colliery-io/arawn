---
id: hooks-integration-wire-the-engine
level: initiative
title: "Hooks integration — wire the engine to fire lifecycle events"
short_code: "ARAWN-I-0056"
created_at: 2026-05-23T03:30:00.000000+00:00
updated_at: 2026-05-23T03:30:00.000000+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: hooks-integration-wire-the-engine
---

# Hooks integration — wire the engine to fire lifecycle events

## Context

The hooks system in `crates/arawn-engine/src/hooks/` (1,873 LOC across 8 files) is **fully built but never integrated**. It ports Claude Code's hook surface: user-configured shell commands that fire at lifecycle moments (PreToolUse, PostToolUse, SessionStart, UserPromptSubmit, …) and can block/allow/modify the event. The infrastructure is solid:

- 25 `HookEvent` variants defined (matches Claude Code's full API surface).
- `HookRunner` orchestrates matching + parallel execution + result aggregation.
- `HookConfig` + `load_merged_hooks(user, project)` reads JSON from `~/.arawn/settings.json` and `<project>/.arawn/settings.json` with merge semantics.
- `CommandHookExecutor` runs hook commands as subprocesses with timeouts.
- `HookMatcher` supports event-type + tool-name + content-pattern matching.

What's missing: **(1)** nothing in production startup loads hooks or attaches a `HookRunner` to the engine, and **(2)** nothing in the engine calls `.fire_hook(...)` at lifecycle moments. The system has zero fire sites. Even if a user wrote a `~/.arawn/settings.json` with hooks, nothing would happen.

This was surfaced during a YAGNI sweep. The operator confirmed hooks are wanted as a real user-facing feature (extend/modify agent behavior via shell commands) — so this initiative wires them in.

## Goals & Non-Goals

**Goals:**

- Initiative target end state: **all 25 `HookEvent` variants fire from real sites in production**.
- V1 lands the ~17 events with natural fire sites in current code. Remaining events stay defined-but-not-fired pending the underlying features they depend on.
- Load hooks at startup from `~/.arawn/settings.json` + `<project>/.arawn/settings.json` via the existing `load_merged_hooks`.
- Hook subprocesses run with `cwd = workstream root` (matches the agent's cwd; lets hook scripts reference project files relative to their own repo).
- Block semantics surfaced cleanly: `PreToolUse` block aborts the tool call with the hook's reason; `UserPromptSubmit` block aborts the turn; etc.
- UAT scenario validates end-to-end: configure a hook → trigger event → verify side effect.
- Docs: a `configure-hooks.md` how-to with the JSON shape and a few examples.

**Non-Goals:**

- Building new trigger sources for events without natural fire sites today. Those are separate initiatives — see deferred list below.
- Inventing a new config format. `load_hooks_from_file` already parses Claude Code-compatible JSON.
- CLI subcommands (`arawn hooks list/validate/dry-run`). Nice-to-have but deferred.

## Detailed Design

### V1 — events fired in this initiative (~17)

Events with natural fire sites in current code:

| Event | Fire site | Block semantics |
|---|---|---|
| `SessionStart` | `LocalService::create_session` | Non-blocking; informational |
| `SessionEnd` | `LocalService` (session close path) | Non-blocking |
| `UserPromptSubmit` | `QueryEngine`, before model call | Block aborts the turn with hook's message |
| `PreToolUse` | `QueryEngine`, before each tool dispatch | Block aborts that tool call; agent sees the reason |
| `PostToolUse` | `QueryEngine`, after successful tool | Non-blocking |
| `PostToolUseFailure` | `QueryEngine`, after tool error | Non-blocking |
| `Stop` | `QueryEngine`, on final response | Non-blocking |
| `StopFailure` | `QueryEngine`, on model stream error | Non-blocking |
| `PreCompact` | `Compactor::compact`, before LLM call | Non-blocking (could block in future if needed) |
| `PostCompact` | `Compactor::compact`, after LLM call | Non-blocking |
| `PermissionRequest` | `PermissionChecker`, when "ask" mode prompts | Non-blocking |
| `PermissionDenied` | `PermissionChecker`, on user denial | Non-blocking |
| `SubagentStart` | `AgentTool`, when sub-agent is spawned | Non-blocking |
| `SubagentStop` | `AgentTool`, on sub-agent completion | Non-blocking |
| `TaskCreated` | `BackgroundTaskManager`, on task create | Non-blocking |
| `TaskCompleted` | `BackgroundTaskManager`, on task complete | Non-blocking |
| `Notification` | `LocalService::notice_tx` broadcast | Non-blocking |

### V2 — events deferred (need new trigger sources)

These 8 events stay defined-but-not-fired until their underlying features exist:

| Event | Missing prerequisite |
|---|---|
| `Setup` | No first-run / initial-setup flow exists |
| `WorktreeCreate` | Git worktree subsystem doesn't exist |
| `WorktreeRemove` | Same |
| `CwdChanged` | CWD isn't tracked as a stateful concept across the agent loop |
| `FileChanged` | No file watcher for user files (HookFileWatcher is for hooks-config reload only) |
| `TeammateIdle` | No teammate-agent system |
| `Elicitation` | No structured-input request flow |
| `PromptInjectionVerdict` | `prompt_injection` module exists in arawn-engine but has no production runner |

Each of these would warrant its own initiative (or be folded into the initiative that builds the missing feature). They are NOT in scope here.

### Startup wire-up (T-A)

- Add a `startup/hooks.rs` (under `crates/arawn/src/startup/`) that:
  - Resolves the user settings path (`~/.arawn/settings.json` or `$ARAWN_DATA_DIR/settings.json`).
  - Resolves the project settings path (`<workstream_root>/.arawn/settings.json`).
  - Calls `load_merged_hooks` to get a `HookConfig`.
  - Constructs a `HookRunner::new(config, workstream_root.clone())` with cwd = workstream root.
  - Wraps in `Arc<HookRunner>` and attaches via `QueryEngine::with_hook_runner(runner)`.
- The `LocalService` also needs a reference for `SessionStart`/`SessionEnd`/`Notification` fires.

### Fire-site pattern

Each fire site follows this shape:

```rust
if let Some(runner) = &self.hook_runner {
    let input = HookInput::pre_tool_use(tool_name, args.clone(), session_id);
    let result = runner.run(&input).await;
    if result.blocked {
        return Err(ToolError::HookBlocked(result.block_reason));
    }
}
```

For non-blocking events, the `result.blocked` branch is omitted (or just logged).

### Error propagation

- Hook subprocess failures (non-zero exit without a structured "block" response) are **non-fatal** — logged and treated as Allow. Don't let a broken user hook break arawn.
- Hook timeouts (>5s default per hook) are treated as Allow with a warning logged.
- Block reasons are surfaced to the agent verbatim in PreToolUse / UserPromptSubmit cases so the model can understand why and adapt.

## Alternatives Considered

- **Rip the hooks system entirely.** Considered during the YAGNI sweep. Operator confirmed hooks are wanted as a real feature, so wiring beats deletion. The 1,873 LOC of infrastructure is well-designed and would have to be rebuilt later from scratch if ripped.
- **Match Claude Code's settings file location (`~/.claude/settings.json`)** for cross-tool config sharing. Rejected — arawn is its own product. Use `~/.arawn/settings.json` and `<project>/.arawn/settings.json`.
- **TOML config format** for consistency with `arawn.toml`. Rejected — `load_hooks_from_file` already parses JSON; matches Claude Code's spec; no value in changing.
- **Wire only the 7-event tool-and-session-lifecycle subset first** (smaller V1). Rejected per operator — the initiative target is the full 25-event surface; V1 fires the 17 wireable subset; the other 8 wait for their underlying features.

## Implementation Plan

Five tasks, sequenced:

- **T-A — Startup loader + attach.** Add `startup/hooks.rs`. Load merged hooks. Attach `HookRunner` to engine + local_service. Pick `cwd = workstream_root`. No fire sites yet — just wire-up. Acceptance: a configured `~/.arawn/settings.json` is observably parsed at startup (log line); unit tests cover the loader.

- **T-B — Tool & turn lifecycle fires.** `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `Stop`, `StopFailure`, `UserPromptSubmit` — 6 events in `QueryEngine`. Block semantics for `PreToolUse` (abort tool call) and `UserPromptSubmit` (abort turn). Unit tests for each fire site.

- **T-C — Session, permission, compaction fires.** `SessionStart`, `SessionEnd`, `PermissionRequest`, `PermissionDenied`, `PreCompact`, `PostCompact`, `Notification` — 7 events in `LocalService` and `Compactor`. Non-blocking for v1.

- **T-D — Subagent + task lifecycle fires.** `SubagentStart`, `SubagentStop` in `AgentTool`; `TaskCreated`, `TaskCompleted` in `BackgroundTaskManager` — 4 events.

- **T-E — Docs + UAT close gate.** New `docs/src/how-to/configure-hooks.md` with the JSON shape and 3 example hooks (auto-format on PostToolUse, audit log on SessionEnd, block bad shell on PreToolUse). New UAT scenario: configure a `PostToolUse` hook on `file_write` that writes a sentinel file in `/tmp`; run a scenario that uses `file_write`; verify the sentinel exists. Full UAT + judge at close.

## Exit Criteria

- [ ] All 5 tasks landed.
- [ ] 17 V1 events fire from real production sites.
- [ ] Startup loads `~/.arawn/settings.json` + `<project>/.arawn/settings.json` and attaches a `HookRunner` to both `QueryEngine` and `LocalService`.
- [ ] Hook subprocesses run with `cwd = workstream root`.
- [ ] `PreToolUse` block aborts the tool call with the hook's reason; `UserPromptSubmit` block aborts the turn.
- [ ] Hook failures (non-zero exit, timeouts) treated as non-fatal Allow + warning log.
- [ ] `docs/src/how-to/configure-hooks.md` exists with the JSON shape and ≥3 example hooks.
- [ ] UAT scenario validates end-to-end (hook configured → triggered → side-effect verified).
- [ ] `cargo check --workspace` clean, `cargo build --workspace --release` clean.
- [ ] `cargo test --workspace --lib` green.
- [ ] `angreal test uat` + `angreal test uat-judge` green at close.
- [ ] 8 V2 events stay defined-but-not-fired with a comment naming the missing prerequisite.

## Related

- ARAWN-V-0001 — vision.
- ARAWN-I-0055 — capability-driven tool filter (prior initiative).
- The YAGNI sweep that surfaced this: [feedback_prefer_config_over_runtime_policy.md] — operator memory about preferring config-driven over runtime policy.
- Future work: separate initiatives for the V2 events' missing prerequisites (worktrees, file watcher, teammate system, etc.).
