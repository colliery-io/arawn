---
id: oauth-distribution-model-shared
level: initiative
title: "OAuth distribution model — shared client vs BYO, per-provider"
short_code: "ARAWN-I-0037"
created_at: 2026-05-06T13:18:44.446529+00:00
updated_at: 2026-05-06T13:18:44.446529+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: oauth-distribution-model-shared
---

# OAuth distribution model — shared client vs BYO, per-provider Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context **[REQUIRED]**

Today every arawn user must register their **own** OAuth app at each provider (Google Cloud Console, Slack api dashboard, Atlassian developer console) and paste `client_id` / `client_secret` into `~/.arawn/arawn.toml` before they can `/connect`. ADR ARAWN-A-0001 documented this as a deliberate v1 trade-off — zero infrastructure for us, but real friction for users.

For an "agent in your terminal" pitch this is a wall most users won't climb:

- **Google verification** alone takes 4–8 weeks plus a security review for sensitive scopes (Gmail, Calendar, Drive). Without verification users see a "this app is unverified" warning and are limited to 100 test users.
- **Slack** requires the app to be in distribution mode (an internal review process) before non-workspace-admins can install it from a non-workspace-owner OAuth client.
- **Atlassian** is the most lenient — public clients work without review — but still requires creating the app + setting redirect URIs.

Claude Desktop, Cursor, ChatGPT plugins, etc. all ship a vendor-owned shared OAuth client. arawn looking like the odd one out here actively hurts adoption.

The technical layer (PKCE flow, callback server, encrypted token storage) is already built and proven via I-0033 and the integration tasks (T-0202 through T-0206). This initiative is purely about the **distribution / trust model**, not the implementation plumbing.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- Decide a per-provider distribution model (BYO, shared, or both) and document the rationale in an ADR that supersedes ARAWN-A-0001.
- For each "shared" provider: register an OAuth app under arawn's brand, complete verification where required, and ship the client_id baked into the binary so users can `/connect <svc>` without any pre-config.
- Keep BYO as a documented fallback for: (a) providers where verification is in flight, (b) users in environments where shared-app consent is denied (e.g. enterprise Google Workspace with strict admin policies), (c) workspaces that require their own app per workspace (e.g. Slack distributed apps).
- Address the security posture of shipping a client_id in a public binary — public-client / PKCE-only flow per provider, where the provider supports it.
- Document operational cost: who owns the OAuth app registrations, who handles verification renewals, who responds when a provider suspends our app.

**Non-Goals:**
- Re-implementing the OAuth runtime (callback server, token storage, refresh) — that's the I-0033 layer and stays as-is.
- Multi-tenant SSO / "log in with arawn" — we're a CLI tool, not an identity provider.
- Cross-device token sharing — out of scope; tokens stay in the local `~/.arawn/tokens/` keychain.
- Slack Enterprise Grid / SSO support — separate concern, defer.

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

{Delete if not a requirements-focused initiative}

### User Requirements
- **User Characteristics**: {Technical background, experience level, etc.}
- **System Functionality**: {What users expect the system to do}
- **User Interfaces**: {How users will interact with the system}

### System Requirements
- **Functional Requirements**: {What the system should do - use unique identifiers}
  - REQ-001: {Functional requirement 1}
  - REQ-002: {Functional requirement 2}
- **Non-Functional Requirements**: {How the system should behave}
  - NFR-001: {Performance requirement}
  - NFR-002: {Security requirement}

## Use Cases **[CONDITIONAL: User-Facing Initiative]**

{Delete if not user-facing}

### Use Case 1: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

### Use Case 2: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

## Architecture **[CONDITIONAL: Technically Complex Initiative]**

{Delete if not technically complex}

### Overview
{High-level architectural approach}

### Component Diagrams
{Describe or link to component diagrams}

### Class Diagrams
{Describe or link to class diagrams - for OOP systems}

### Sequence Diagrams
{Describe or link to sequence diagrams - for interaction flows}

### Deployment Diagrams
{Describe or link to deployment diagrams - for infrastructure}

## Detailed Design **[REQUIRED]**

### Per-provider matrix (proposed; to be locked in design phase)

| Provider | Recommended model | Rationale | Cost / effort |
|---|---|---|---|
| **Google** (Gmail / Calendar / Drive) | **Shared, PKCE public client + verified** | Gmail/Drive are the marquee use cases; verification is mandatory for sensitive scopes. PKCE means we don't ship a real secret. | $0 reg fee but security review (~$300 contractor cost) + 4–8 weeks. Annual re-verification. |
| **Atlassian** (Jira / Confluence) | **Shared, public client (3LO)** | Atlassian explicitly supports public clients with PKCE; no verification required. | Half-day registration, no ongoing review. |
| **Slack** | **BYO only (for now)** | Distribution mode review is unpredictable; per-workspace install model fights our "personal assistant" framing; multi-workspace already deferred to I-0034. | Revisit after I-0034 lands. |
| **GitHub / GitLab** (future) | **Shared, public client** | Both providers support installation-style apps with no verification; well-trodden path. | Half-day per provider when we add the integration. |
| **Anthropic / OpenAI keys** | **Always BYO** | Not OAuth — these are static API keys; user owns billing. | N/A. |

### Resolution order in code

When `/connect <svc>` runs:

1. Check `[integrations.<svc>] client_id`/`client_secret` in `arawn.toml`. If present, use them (BYO path).
2. Else check the **bundled shared client** for that provider (compiled-in `client_id`, optional `client_secret` for confidential-client providers). If present, use it (shared path).
3. Else error: "no shared OAuth client is configured for this build; either upgrade arawn or provide your own credentials."

This means BYO config silently overrides the shared client — useful for users on Google Workspace where their admin has blocked our shared app.

### Public-client / PKCE-only design

For providers where we use the shared client, the binary ships **only** the `client_id` (publicly known by design). The `client_secret` is either:

- **Omitted entirely** (Google + Atlassian both support PKCE-only public clients for these scopes).
- **Replaced by per-installation client_secret** (a server-side mint endpoint we expose at `auth.arawn.dev`, returning a short-lived per-flow secret) — only used if a provider mandates a confidential client.

Realistically, with PKCE public clients we don't need any server. The shared `client_id` is just a constant in the binary.

### Brand / consent screen

The OAuth consent screen will display:

> **arawn** wants access to your Gmail.
> By <Dylan Storey or future legal entity>.

This makes us the user-visible owner of the integration. Implications:

- Provider abuse: if any user does something objectionable through our OAuth app (mass spam via Gmail, etc.) Google can suspend the entire app, breaking it for every arawn user. Mitigation: clear ToS / acceptable-use policy bundled with the app + monitoring on token usage rates from the runtime.
- Privacy policy + ToS: required for verification. We need a public URL hosting both.
- Trademark: "arawn" the word is generic-ish; defensive registration is out of scope but worth noting.

### BYO fallback UX

When `[integrations.<svc>]` is missing in arawn.toml, the TUI's `/connect <svc>` flow uses the shared client. When it's present, we use BYO. **Document this clearly in the integration help text** so a user pasting BYO credentials doesn't accidentally override and wonder why their consent screen shows their own app name.

### Rotation / kill-switch

A bundled shared `client_id` cannot be rotated without shipping a new arawn binary. If a provider suspends the app or we need to rotate, **all users on the old binary lose integration access** until they upgrade. Mitigation:

- Server-side health check at startup: `GET https://auth.arawn.dev/oauth-status` returns `{ providers: { google: "ok" | "suspended" } }`. TUI surfaces a sticky banner: "Google integration is currently suspended; please upgrade arawn." (Reuses the heartbeat-row plumbing from T-0212.)
- For "ok" responses we proceed silently. If the endpoint is unreachable, fail open (assume ok) rather than break offline use.

### Cost / ownership table

| Item | One-time | Recurring |
|---|---|---|
| Google verification (security review contractor) | ~$300 | every 12 months on policy change |
| Domain `auth.arawn.dev` (status endpoint host) | ~$15/yr | $5/mo Cloud Run / Fly.io |
| Privacy policy / ToS hosting | $0 (GitHub Pages) | $0 |
| App registration time | 4 hours per provider | minor — respond to provider compliance pings |



## UI/UX Design **[CONDITIONAL: Frontend Initiative]**

{Delete if no UI components}

### User Interface Mockups
{Describe or link to UI mockups}

### User Flows
{Describe key user interaction flows}

### Design System Integration
{How this fits with existing design patterns}

## Testing Strategy **[CONDITIONAL: Separate Testing Initiative]**

{Delete if covered by separate testing initiative}

### Unit Testing
- **Strategy**: {Approach to unit testing}
- **Coverage Target**: {Expected coverage percentage}
- **Tools**: {Testing frameworks and tools}

### Integration Testing
- **Strategy**: {Approach to integration testing}
- **Test Environment**: {Where integration tests run}
- **Data Management**: {Test data strategy}

### System Testing
- **Strategy**: {End-to-end testing approach}
- **User Acceptance**: {How UAT will be conducted}
- **Performance Testing**: {Load and stress testing}

### Test Selection
{Criteria for determining what to test}

### Bug Tracking
{How defects will be managed and prioritized}

## Alternatives Considered **[REQUIRED]**

**A. Stay BYO for everything (status quo).** Rejected for the marquee providers (Google, Atlassian) because the friction kills adoption — users won't sit through a Google Cloud Console wizard before sending their first agent prompt. Acceptable for Slack while multi-workspace is deferred.

**B. Shared client for everything, no BYO fallback.** Rejected because it removes the escape hatch for: (a) Google Workspace tenants where admins block third-party OAuth apps, (b) users who want their own consent-screen branding (e.g. shipping arawn as a re-branded internal tool), (c) early-adopter periods before our shared app is verified.

**C. Shared client + central token broker.** Rejected. We could host a server at `auth.arawn.dev` that mints tokens and proxies them to users, never exposing the OAuth client to the binary at all. But: (1) we become a critical dependency for offline use, (2) we'd hold every user's tokens server-side, which is a compliance / privacy nightmare, (3) users on isolated networks can't use arawn integrations at all. Local-only token storage is a load-bearing principle.

**D. Per-tier shared client (free vs paid).** Rejected as premature; we don't have a paid tier and won't for the foreseeable future. Revisit if/when commercialization happens.

## Implementation Plan **[REQUIRED]**

This work splits into discrete phases. Each phase is a task; phase 1 unblocks 2 and 3 in parallel, and phase 4 ships the result.

### Phase 1 — Resolution-order plumbing (T-TBD)
Refactor `arawn-integrations` so the OAuth-client lookup walks: BYO config → bundled shared → error. Today the code reads `[integrations.<svc>]` directly; after this phase it asks an `OAuthClientResolver` that knows about both sources. Pure refactor — no shared client registered yet, no functional change for existing BYO users.

**Acceptance:** existing BYO setups still work; new `bundled_clients` registry is empty but pluggable; unit tests cover the resolution priority.

### Phase 2 — Atlassian shared client (T-TBD)
Easiest provider. Register an arawn-owned Atlassian Cloud OAuth app (3LO), bake the `client_id` into the binary, ship via Phase 1's resolver. Atlassian PKCE-only public clients work without verification.

**Acceptance:** `/connect atlassian` works on a fresh install with no `arawn.toml` integrations block. BYO still overrides.

### Phase 3 — Google verification (T-TBD)
Hardest provider. Submit Gmail / Calendar / Drive scopes for Google's verification process. Steps:

1. Stand up `auth.arawn.dev` with a privacy policy + ToS page.
2. Configure the existing Google Cloud project as "external" with our scope justifications.
3. Submit to verification with a security assessor (CASA Tier 2 for sensitive scopes).
4. Address review feedback (typical 2–3 rounds).
5. On approval, bake the verified `client_id` into the binary via Phase 1's resolver.

**Acceptance:** consent screen shows "arawn" with no unverified-app warning; `/connect gmail` works on a fresh install. Annual renewal calendar reminder set.

### Phase 4 — Operational kill-switch (T-TBD)
Stand up `auth.arawn.dev/oauth-status` and wire the TUI to read it on startup. Sticky banner if any shared client is suspended. Reuses T-0212 heartbeat row.

**Acceptance:** flipping a flag on the status endpoint produces a banner in the TUI within one minute of the next session start. Failing open (endpoint unreachable) leaves the TUI silent.

### Phase 5 — ADR + docs (T-TBD)
Write ADR ARAWN-A-0002 superseding A-0001 with the per-provider model. Update `docs/integrations.md` to explain shared vs BYO and when to choose each.

**Acceptance:** ADR merged with "decided" status; integration docs cover both paths with decision-tree.

### Sequencing

Phase 1 must land first; 2 and 3 run in parallel after that (Atlassian completes in days, Google in weeks). Phase 4 can start any time after Phase 1. Phase 5 runs last.

### Open questions to resolve in design phase (before decomposing)

1. **Legal entity for the OAuth app registration.** Personal name vs LLC vs DBA — has tax / liability implications. Currently leaning personal until commercial use.
2. **Privacy policy contents.** Does our local-only-storage architecture even need the standard "we collect" boilerplate, or can we explicitly disclaim data collection?
3. **`auth.arawn.dev` hosting.** Cloudflare Pages vs Fly.io vs Cloud Run. ~$5/mo either way; need to actually own arawn.dev first.
4. **arawn.dev domain.** Is it available? If not, fallback name?
5. **Slack revisit timing.** Hold off until I-0034 (multi-workspace) is decomposed, or take the BYO path indefinitely?