# Identity by workstream

*Explanation. Why arawn's persona is a workstream attribute. ARAWN-I-0035's design.*

A few months into building arawn, three parallel design reviews (visual, interaction, identity) all converged on the same root finding: arawn read like a coding REPL with OAuth buttons bolted on, not the personal agentic assistant the vision describes. The system prompt opened with the assistant frame and then immediately reframed everything in terms of software engineering tasks. The CLI's `--help` literally said "LLM-powered coding assistant." The status bar surfaced model + token usage, never the user's life.

The fix could have been a one-shot rewrite — change the system prompt, change the `about` string, ship. But the project legitimately needs both personas: some users want the coding tool, some users want the personal assistant. The right primitive turned out to be making **persona a workstream attribute**, not a global setting.

This is ARAWN-I-0035. The Phase 1 changes (system prompt rewrite, CLI metadata, workstream `identity_profile` column) shipped 2026-05-18.

## The `identity_profile` column

Each workstream now carries an `identity_profile` field (source: `crates/arawn-core/src/workstream.rs:137`):

```rust
pub enum IdentityProfile {
    #[default]
    Assistant,
    Coding,
}
```

Persisted in SQLite as the lowercase string in `workstreams.identity_profile`. Default is `Assistant` for every new workstream. The constructor never validates — anything else round-trips to `Assistant`. (Default-safe: a future v3 enum variant doesn't break v2 readers.)

V10 migration (`crates/arawn-storage/migrations/V10__workstream_identity_profile.sql`) added the column to existing workstreams. Pre-V10 workstreams default to `assistant` on first read after the migration.

## How the prompt switches

`crates/arawn-engine/src/system_prompt.rs` defines two const sets:

- `ASSISTANT_IDENTITY` / `ASSISTANT_DOING_TASKS` / `ASSISTANT_WORK_PROTOCOL`
- `CODING_IDENTITY` / `CODING_DOING_TASKS` / `CODING_WORK_PROTOCOL`

`SystemPromptBuilder::load_static_sections` reads the workstream's `IdentityProfile` and emits the corresponding set. The remaining sections (system, actions, using_tools, tone, output_efficiency) are persona-neutral and shared.

The persona switch happens at engine construction time (per-session), not per-turn. Switching workstreams creates a new session config; the new config rebuilds the prompt with the new persona.

`LocalService::build_engine_config` is the wiring — reads workstream metadata, picks `IdentityProfile`, hands it to the prompt builder. No new tools, no new RPC; the existing engine just gets a different prompt.

## Why `Assistant` is the default

The vision is explicit: *"a personal agentic assistant that helps you stay organized and on top of life. arawn watches, checks, summarizes, and nudges — so you don't have to keep everything in your head."* The coding-tool persona was an accident of how the prompts evolved, not an explicit design choice.

Making `Assistant` the default — even for workstreams that sound coding-flavored — forces the opt-in to be deliberate. If you create a workstream called `arawn-dev`, you'll have to explicitly say "make this a coding workstream" via `workstream_describe { identity_profile: "coding" }`. The friction is intentional: it nudges new users toward the vision's intended frame.

## Why per-workstream, not per-session or per-message

Three alternatives were considered:

### Per-session

Each session would carry its own persona. Tempting because sessions are visible (`/session list`), but the unit of *purpose* in arawn is the workstream — sessions are a sub-unit of a workstream. Letting sessions diverge from their workstream's persona would mean a `personal` workstream could host a `coding` session, which is incoherent.

### Per-message

Slash commands like `/persona coding` would flip persona for the next turn. Maximum flexibility but maximum confusion: the agent's behavior would change unpredictably mid-conversation. Rejected.

### Per-workstream

The accepted choice. Workstreams are the unit of purpose. A `personal` workstream is unambiguously for life-assistant work; a `code/arawn` workstream is unambiguously for engineering. The persona follows.

## What changes between personas

The `ASSISTANT_*` prompts emphasize:

- *Read before you act.* When something exists (a thread, a ticket, a calendar invite), look at it before suggesting changes.
- *Don't fabricate.* If a tool returns no results, retry with broader terms before reporting empty. Never fall back to training-data knowledge to fill a gap.
- *Be careful with actions that send messages, schedule events, or modify external state.* Confirm before doing them unless the user has clearly authorized you for this turn.
- *Don't add scope.* "Summarize my inbox" doesn't need follow-ups drafted. "What's on my calendar" doesn't need rescheduling proposed.

The `CODING_*` prompts emphasize:

- *Read existing code before suggesting modifications.*
- *Don't create files unless absolutely necessary; prefer editing existing files.*
- *Don't add features, refactor, or make "improvements" beyond what was asked.* A bug fix doesn't need surrounding code cleaned up.
- *Don't add error handling or validation for scenarios that can't happen.*

Both share the same `# Error recovery` and `# Behavioral context (arawn.md)` sections — those are persona-neutral.

## How to switch

```text
workstream_describe { "identity_profile": "coding" }
```

Or, if you prefer SQL surgery:

```sql
UPDATE workstreams SET identity_profile = 'coding' WHERE name = 'arawn-dev';
```

Either way, take effect on the next session in that workstream. Active sessions don't re-roll the prompt; close and reopen to pick up the change.

## What's NOT included in `identity_profile`

The system prompt isn't the only persona-shaped thing in arawn. Three things were considered and deferred:

- **Status bar widgets.** A coding workstream's status bar arguably wants token usage + model. An assistant workstream's wants unread counts + next calendar item + alerts. The status bar redesign is Phase 3 of I-0035, gated on I-0036 (TUI visual coherence).
- **Default workflows.** The assistant persona implies a set of default workflows (morning brief, weekly ceremony, retro). The coding persona implies fewer. Default-workflow seeding is a separate I-0033 follow-up.
- **CLI `about` string.** Phase 1 changed this from "LLM-powered coding assistant" to "Personal agentic assistant — watch, check, summarize, and nudge across your tools." Singular for the binary; per-workstream personas don't apply at this layer.

## Status

Phase 1 (this) shipped 2026-05-18. Phases 2-4 (briefing service, TUI dashboard surfaces, notification toasts) remain.

## Related

- [Workstreams explanation](./workstreams.md) — what a workstream is.
- [Workstream CLI reference](../reference/workstream-cli.md) — `identity_profile` field + how to set it.
- [What is arawn?](./what-is-arawn.md) — the vision the persona supports.
- ARAWN-I-0035 (in `.metis/initiatives/`) — the design doc this page summarizes.
