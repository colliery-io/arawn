---
id: multi-workspace-slack-per
level: initiative
title: "Multi-workspace Slack — per-workspace integration instances"
short_code: "ARAWN-I-0034"
created_at: 2026-05-05T15:54:45.376435+00:00
updated_at: 2026-05-05T15:54:45.376435+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: multi-workspace-slack-per
---

# Multi-workspace Slack — per-workspace integration instances

## Context

Today's Slack integration ([ARAWN-I-0033](../ARAWN-I-0033/initiative.md), specifically [ARAWN-T-0204](../ARAWN-I-0033/tasks/ARAWN-T-0204.md)) is hard-wired to a single workspace:

- One `SlackIntegration` struct holding one `client_id` / `client_secret` pair from `ARAWN_SLACK_CLIENT_ID` / `_SECRET` env vars.
- One Slack-keyed slot in `arawn_auth::TokenStore` — second OAuth overwrites the first.
- Tools take `Arc<SlackIntegration>` with no workspace selector.

For users who participate in multiple Slack workspaces (work, side projects, communities), this means they pick one and lose the others. The agent's value as a "personal assistant across the systems I run my day from" caps at whichever workspace they connected last.

This initiative resolves that by making each workspace its own first-class registered integration — `slack@acme`, `slack@personal`, etc. — each with its own token, scopes, and tool set. This was option **C** in the design discussion (per the chat thread that produced this initiative); options A (active-profile pointer) and B (per-call workspace param) were considered and rejected — see Alternatives.

## Goals & Non-Goals

**Goals:**
- Connect to N Slack workspaces simultaneously, where N ≥ 2 in the typical case.
- Each workspace is independently `/connect`-able, `/disconnect`-able, and revocable, without affecting the others.
- The agent can call tools against a specific workspace ("post to #engineering on acme") without ambiguity.
- Granted scopes are tracked per workspace — workspace A can have `chat:write.public` while B doesn't.
- The system prompt fragment from `Integration::capabilities_summary` lists each connected workspace separately, so the agent knows which workspaces are available and what each can do.

**Non-Goals:**
- Cross-workspace tool calls in a single invocation (e.g. one `slack_post` that fans out to multiple workspaces). Agent makes N calls if it wants N posts.
- Multi-account support for Gmail / Calendar — those are separate refactors with different shapes (Google's OAuth / account model is different).
- A UI for managing workspace aliases beyond `/connect slack@<alias>` and `/disconnect slack@<alias>`.
- Renaming workspaces post-connect. Reconnect with a new alias instead.

## Requirements

### User Requirements
- The user can connect to a second Slack workspace without disconnecting the first.
- After connecting workspace `acme`, calling `/integrations` shows `slack@acme ✓` distinct from any other Slack entries.
- Tool names visible to the agent are clearly scoped per workspace so cross-workspace calls don't collide.
- Disconnecting one workspace must not affect the others' tokens or scopes.

### System Requirements
- **REQ-001:** `Integration` registry can hold multiple integrations whose service-name prefix collides (`slack@a`, `slack@b`).
- **REQ-002:** TokenStore keys remain unique across workspaces — the existing arbitrary-string-key API supports this; the only change is the key format.
- **REQ-003:** Each per-workspace integration carries its own granted-scope set, queried by the per-tool scope check from Phase 1 of T-0204's scope-aware work.
- **REQ-004:** The `capabilities_summary` impl returns one summary per connected workspace; the engine's per-turn prompt builder lists them together.
- **REQ-005:** OAuth client_id / client_secret are shared across workspace installs (one Slack app, multiple installs is the supported pattern). Per-workspace client credentials are out of scope.
- **NFR-001:** Adding a new workspace must not require a server restart. `/connect slack@<alias>` is sufficient.
- **NFR-002:** No measurable per-tool-call overhead for the single-workspace case (the common case must stay fast).

## Use Cases

### Use Case 1: User adds a second workspace

- **Actor:** User who already has Slack workspace `acme` connected.
- **Scenario:**
  1. User adds the same Slack app (`ARAWN_SLACK_CLIENT_ID/_SECRET`) to a second workspace via Slack's "Install to another workspace" flow.
  2. User runs `/connect slack@personal`.
  3. arawn opens the OAuth browser flow; Slack's UI lets the user pick the workspace to install into.
  4. On callback, arawn persists the new token at `slack:personal` and registers a new tool set.
  5. Server emits a `ServerNotice` confirming the second workspace.
- **Expected Outcome:** `/integrations` shows both workspaces ✓; tools for both are available to the agent.

### Use Case 2: Agent posts to a specific workspace

- **Actor:** The LLM, given user prompt "tell #ops on acme that the deploy succeeded."
- **Scenario:**
  1. Agent reads the system prompt fragment listing `slack@acme` and `slack@personal` with their granted scopes.
  2. Agent invokes `slack_acme_post({channel: "#ops", text: "deploy succeeded"})` — the workspace-scoped tool name disambiguates.
  3. Tool resolves the per-workspace `SlackIntegration`, executes against that workspace's token.
- **Expected Outcome:** The message lands on `acme`, not `personal`.

### Use Case 3: User disconnects one workspace

- **Actor:** User leaves `personal`.
- **Scenario:**
  1. `/disconnect slack@personal`.
  2. arawn deletes the `slack:personal` token; deregisters the per-workspace tools.
- **Expected Outcome:** `/integrations` shows only `slack@acme ✓`. Tools for `personal` no longer appear in the agent's tool list. `acme`'s state is untouched.

## Architecture

### Overview

The cleanest split is to **stop having "Slack" as a singleton integration**. Instead, the integration registry holds N instances of `SlackIntegration`, each named `slack@<alias>` (or `slack:<alias>` — bikeshed during implementation). Each instance owns:

- Its own `data_dir` subpath (`<data>/integrations/slack-<alias>/`)
- Its own `TokenStore` key (`slack:<alias>`)
- Its own granted-scope set (read from its token)
- Its own client_id / client_secret pair (cloned from the shared Slack app config — same OAuth app, different installs)

Tools are registered N times, with workspace-scoped names: `slack_acme_list_channels`, `slack_personal_list_channels`, etc. Each tool instance holds an `Arc<SlackIntegration>` pointing at its specific workspace.

### Component Diagram (sketch)

```
LocalService
 └── integration_registry: HashMap<String, Arc<dyn Integration>>
      ├── "slack@acme"     → SlackIntegration { data_dir/integrations/slack-acme/, ... }
      ├── "slack@personal" → SlackIntegration { data_dir/integrations/slack-personal/, ... }
      └── ...

ToolRegistry
 ├── slack_acme_list_channels     → Arc<SlackIntegration[acme]>
 ├── slack_acme_post              → Arc<SlackIntegration[acme]>
 ├── slack_personal_list_channels → Arc<SlackIntegration[personal]>
 └── ...
```

### Key Design Calls

- **Workspace alias source:** Slack's OAuth response includes `team.id` and `team.name`. Use a sanitized `team.name` as the alias by default (`acme-corp` → `acme-corp`); fall back to `team.id` if the name is empty or non-ASCII. Stored alongside the token. User can override via `/connect slack@<alias>`.
- **Bootstrap discovery:** On server startup, scan `<data>/integrations/slack-*/` and register each as a separate `SlackIntegration`. No need for the user to re-add workspaces between restarts.
- **Tool registration timing:** Per-workspace tools register at server startup (for already-connected workspaces) and on `/connect slack@<alias>` (for new ones). De-register on `/disconnect`.
- **`/integrations` listing:** One row per workspace, e.g. `slack@acme ✓ (12 scopes)`.

## Detailed Design

### Integration trait — no changes

`Integration::name()` already returns `&str`; we just start using values like `slack@acme` instead of `slack`. The trait does not need to know that we have multiple Slack instances.

### `SlackIntegration` — modest refactor

Constructor changes from:
```
SlackIntegration::new(data_dir, client_id, client_secret)
```
to:
```
SlackIntegration::new(data_dir, alias: String, client_id, client_secret)
  // self.name = format!("slack@{alias}")
  // self.token_key = format!("slack:{alias}")
  // self.creds_dir = data_dir.join(format!("integrations/slack-{alias}"))
```

The OAuth flow itself is unchanged — still PKCE through `arawn-auth::OAuthClient`. The redirect-URI fixed-port mode (8080 default) is shared across workspaces; only one `/connect slack@<alias>` runs at a time.

### Tool naming

Tools get a name prefix per workspace. Either:
- `slack_acme_list_channels` (alias inline in name) — agent-friendly, easy to read
- `slack_list_channels:acme` (alias as suffix) — collides with how the LLM tooling typically formats names; rejected
- `slack_list_channels` always, with a `workspace` arg — that's option B (rejected); pulling it back here would defeat the purpose

Pick: alias inline. Each tool struct gets a `workspace_alias: String` field used both for `name()` and for resolving the integration.

### Tool list explosion (real cost)

6 tools × N workspaces = 6N entries in the LLM's tool list. For N=2 that's 12, fine. For N=5 that's 30, getting noisy. Mitigation: don't register tools for workspaces that aren't actively connected (we already know which are connected via the `is_connected` check at registry time).

### `/connect` flow changes

`/connect slack` (no alias) → error message asking which alias to use, suggesting `/connect slack@<your-workspace-name>`.
`/connect slack@<alias>` → check if `<alias>` already exists in registry (already connected: refuse with "already connected, /disconnect first"); else create a new `SlackIntegration` with that alias, run OAuth, persist, register, emit notice, register tools.

### Deserializing the alias from OAuth

After OAuth completes, slack-morphism's response carries `team.id` and `team.name`. Persist both alongside the token. If the user supplied an explicit alias to `/connect`, use that; else use sanitized `team.name`.

### Migration of existing single-workspace installs

The current install has tokens at TokenStore key `slack` and credentials in `<data>/integrations/slack/`. On startup, if we see this old layout, migrate it:
- Move `<data>/integrations/slack/` → `<data>/integrations/slack-<inferred-alias>/`
- Rename TokenStore key `slack` → `slack:<inferred-alias>`
- Inferred alias from the token's `team.id` if available; else `default`

One-time, idempotent, with a notice on completion. After this, the `slack` (no-alias) name is reserved as an error case.

## Alternatives Considered

### Option A — Active-profile pointer (rejected)

A single `SlackIntegration` instance holding a `HashMap<alias, Token>` plus a "currently active" pointer flipped by `/use slack <alias>`. Tools always read the active token.

- **Pro:** Smallest refactor (~80 LOC).
- **Con:** Agent can never query two workspaces in the same turn — the killer flaw. "What did engineering at acme decide vs at personal" requires explicit user toggling, which defeats the agent UX.

### Option B — Per-call workspace parameter (rejected)

Single `SlackIntegration` with a token map; every Slack tool gets a `workspace: string` parameter. Agent can call into either workspace any time.

- **Pro:** No tool-name explosion.
- **Pro:** Token-map model is what most multi-tenant agent integrations land on (Linear's MCP, Notion's, etc.).
- **Con:** Every tool's parameter schema gets a `workspace` arg the LLM has to remember to pass. The Phase 2 capabilities_summary work helps surface what's available, but the agent still has to thread the param through every call.
- **Real reason for rejection:** the user explicitly picked C in the design conversation. The reasoning being that workspace-scoped tool names are clearer for the LLM than scope-laundering through a parameter, and that the per-workspace isolation extends naturally to differing scope sets per workspace (workspace A might grant `chat:write.public`, workspace B might not).

If implementation cost of C balloons during execution, falling back to B is cheap — the changes are largely additive.

### Option D — Multiple Slack apps with separate client credentials (rejected)

`ARAWN_SLACK_CLIENT_ID_ACME`, `ARAWN_SLACK_CLIENT_SECRET_ACME`, etc. Each workspace runs through a different Slack app.

- **Con:** Operational nightmare — user has to register one Slack app per workspace.
- **Con:** Multi-install of one Slack app already solves the auth problem; this just adds setup tax.

## Implementation Plan

Decompose into tasks at design-phase exit. Likely shape:

1. **Foundation:** Refactor `SlackIntegration::new` to take an `alias`, derive name + token key + dir from it. Update existing single-workspace flow to pass an alias of `default` (or migrate from team.id at first OAuth completion). Maintain green tests.
2. **Bootstrap discovery:** Scan `<data>/integrations/slack-*/` at startup, register each as a separate `SlackIntegration`. Verify `/integrations` lists each.
3. **Tool naming + multi-registration:** Tools take a `workspace_alias`; their `name()` includes it. Registry registers N copies of each tool (one per workspace).
4. **OAuth alias derivation + `/connect slack@<alias>`:** Pull `team.id` / `team.name` from the OAuth response, store alongside token, surface in capabilities_summary. Error path for `/connect slack` (no alias).
5. **Migration:** One-time auto-migration of pre-multi-workspace installs.
6. **Docs:** Update `docs/src/integrations/slack.md` with the multi-workspace model and the alias convention.

Estimated complexity: **L** (~400 LOC plus a meaningful test surface for the multi-instance behavior).

## Status Updates

### 2026-05-05 — Filed (discovery)

Initiative captured from a smoke-test conversation while validating the Slack integration's first end-to-end live use. The single-workspace assumption is baked in deeply enough that this needs its own initiative rather than a bug fix on T-0204. Estimated complexity L; not yet decomposed. Next step is moving to design phase and selecting which task slices to cut.