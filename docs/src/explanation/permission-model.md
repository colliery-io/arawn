# Permission model

*Explanation. Why deny > allow > ask, why plan mode exists, why the modes are coarse, what the audit log is for.*

arawn lets an LLM run shell commands and edit files on your machine. That's a lot of trust. The permission model is what scopes the trust — turning "this LLM has full access" into "this LLM has rules-bounded access."

For the rule syntax and evaluation reference, see [permissions reference](../reference/permissions.md). For preset configurations, see [lock down permissions how-to](../how-to/lock-down-permissions.md). This page is about the design.

## Why deny > allow > ask

The evaluation order is:

```
deny  >  allow  >  ask  >  mode fallback
```

Why deny first? Three reasons:

### 1. Safety asymmetry

A wrong "allow" runs something unintended. A wrong "deny" denies something safe. The cost of false-allow is much higher than false-deny. Putting deny first means the rules you wrote to prevent specific bad outcomes always win.

### 2. Composition

If `allow > deny`, you'd write `allow = ["shell(*)"]` and then have to write a thousand specific denies to claw back. With `deny > allow`, you write `allow = ["shell(git *)"]` for the safe set and `deny = ["shell(*--force*)"]` for the carve-out — small, focused, and the carve-out wins.

### 3. Explicitness

Deny is the strong statement. "I never want X to happen, no matter what other rules say." Allow is permissive. Ask is "I'll decide in the moment." Ordering by strength matches user intent: the strongest statement is what holds.

## Why "ask" exists

Allow + deny would cover every case logically. Why have ask?

Because some calls are situational. Editing a Cargo.toml might be fine when you're working on a Rust file. The same edit on a foreign repo's Cargo.toml might be wrong. You don't want a static rule for that — you want the moment-of-truth choice.

Ask sits between allow and deny: the rule (or mode) says "let me decide each time." The TUI prompts. Your response is `AllowOnce` / `AllowAlways` / `Deny`. Session-grant caching makes `AllowAlways` persistent for the session without writing to TOML.

## Why four modes, not a flat scale

The four modes — `default`, `accept_edits`, `bypass`, `plan` — correspond to four distinct postures:

| Mode | ReadOnly | FileWrite | Shell | Other |
|---|---|---|---|---|
| `default` | allow | ask | ask | ask |
| `accept_edits` | allow | allow | ask | ask |
| `bypass` | allow | allow | allow | allow |
| `plan` | allow | deny | deny | deny |

- **`default`** — you're at the keyboard; you're OK with the agent reading but you want to gate writes. Safe starting point.
- **`accept_edits`** — you've decided "this whole session, the agent should be able to edit." Useful for "let's refactor the whole crate" sessions where every prompt would be a write.
- **`bypass`** — unattended runs. CI. A sandboxed VM. You've decided you trust the rules and don't want any prompts.
- **`plan`** — investigation only. Every side-effect tool is **denied** (not asked). The agent reads, thinks, plans — doesn't act. Toggle in via `/plan`; the agent uses `enter_plan_mode` / `exit_plan_mode` to toggle out.

The four modes are coarse because finer-grained modes don't add much. A "writes but ask for delete" mode would mean designing a third write-category — fiddly without much win. Use rules for that level of nuance.

## Why plan mode denies instead of asks

Plan mode is the one mode where the default for write/shell is *deny*, not *ask*. Why?

When you flip to plan, the intent is "I want the agent to think, not act." If write tools merely *asked*, every prompt would interrupt the planning. By denying outright, the agent learns plan mode at the start of the turn: it tries `file_write`, sees "denied (plan mode)", and adapts — switches to `think` for note-taking, asks the user for inputs instead of editing files.

The two mode-toggle tools (`enter_plan_mode`, `exit_plan_mode`) are exempt from the deny — they're the agent's way to leave plan mode. Without that exemption you'd have a one-way trap.

## The rule format, briefly

```toml
[permissions]
allow = [
    "Read",                 # exact tool name
    "file_*",               # glob on tool name
    "shell(git *)",         # tool name + content pattern
    "shell(cargo *)",
]
deny = [
    "shell(rm -rf *)",
    "shell(curl *)",
]
ask = [
    "web_fetch",
]
```

The tool name matches exact or glob. If the rule has a `(content pattern)`, the tool's first-positional-string argument must also match that glob. Three pattern targets:

- `shell(...)` matches against the shell command string.
- `web_fetch(...)` matches against the URL.
- `Read(...)` / `file_*(...)` match against the path.

The pattern syntax is glob, not regex — `*` and `?`. The reasoning: globs are the lowest-common-denominator pattern language, easy to write, predictable. Power users who want regex can use deny lists more aggressively.

## Session grants vs. TOML rules

When you respond `AllowAlways` to a prompt, arawn writes a session grant — an in-memory rule that auto-allows future calls of the same shape. The grant lives for the session only.

Why not persist to TOML?

- **You answered in a moment of context.** "Yes always allow this `git pull`" is a session-scoped trust, not a forever-trust. Persisting it would surprise you later.
- **TOML edits are deliberate.** Editing `arawn.toml` should be a "I'm reconfiguring permanent posture" action, not a side-effect of a prompt click.

If you find yourself granting the same thing every session, that's a signal to write it into TOML. The TUI (work in progress) will eventually surface frequent grants as TOML suggestions.

## Why the sandbox is separate

Permission rules are policy. The shell sandbox (sandbox-exec on macOS, bubblewrap on Linux) is enforcement at the syscall level. They compose:

- Rule says "allow `shell(rm -rf node_modules)`."
- Sandbox sees the command tries to read `~/.ssh/id_rsa` somewhere in its execution: syscall fails.

The sandbox is the *floor* — what the agent *can* do at the OS level no matter what rules say. Rules are above it.

Why two layers? Defense in depth. A misconfigured rule shouldn't be the only thing standing between the agent and `/etc/shadow`. See [shell sandbox reference](../reference/shell-sandbox.md).

## The audit log

Every allow / deny / ask decision is logged. `/permissions` shows the current rule set, mode, and recent decisions. The server log records the full audit at `DEBUG` level.

Why audit?

- **Debugging.** "Why did the agent fail to write that file?" → permission denied; here's the rule that matched.
- **Trust calibration.** Reviewing what got allowed in a session lets you decide whether to tighten rules.
- **Forensics.** If something went wrong, the log shows what the agent did with permission and what it tried but couldn't.

## What's NOT in the permission model

- **Network rate limits.** The sandbox's network-tool allowlist gates which binaries can touch the network; there's no per-call rate limiter. Rate limiting is the provider's job.
- **Per-user permissions.** arawn is single-user. There's no "Alice can do X but Bob can't."
- **Time-based rules.** No "deny shell at night." Use modes (`/plan` before bed) instead.
- **Approval delegation.** Permission prompts go to the running TUI session. There's no "send the approval request to my phone" path.

These constraints are intentional. The model is small enough to reason about. Adding more dimensions would make rule-debugging painful.

## Related

- [Permissions reference](../reference/permissions.md) — rule syntax + responses + audit.
- [Shell sandbox reference](../reference/shell-sandbox.md) — the floor underneath.
- [Lock down permissions how-to](../how-to/lock-down-permissions.md) — three preset configs.
- [The agent loop](./the-agent-loop.md) — where permission checks fit in a turn.
