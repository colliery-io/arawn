---
id: integration-setup-docs-first-run
level: initiative
title: "Integration setup docs — first-run OAuth walkthroughs that actually work"
short_code: "ARAWN-I-0038"
created_at: 2026-05-06T13:44:20.906686+00:00
updated_at: 2026-05-06T13:44:20.906686+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: integration-setup-docs-first-run
---

# Integration setup docs — first-run OAuth walkthroughs that actually work Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context **[REQUIRED]**

The OAuth setup story is the single biggest adoption barrier for arawn integrations. ARAWN-I-0037 documents the long-term fix (shared client + verification), but until that lands we're BYO — and BYO setup is brutal.

The existing docs at `docs/src/integrations/{gmail,calendar,slack}.md` are stale in three load-bearing ways:

1. **They predate the arawn.toml integrations block.** Every doc tells users to set `ARAWN_GMAIL_CLIENT_ID` / `_SECRET` env vars. The TOML path (resolution: env → `[integrations.<svc>]` → `[integrations.google]` shared) isn't mentioned. Users following the docs end up with a less-good experience than the TOML path enables.

2. **They predate Google's "Google Auth Platform" UI rework.** The Gmail doc says "APIs & Services → OAuth consent screen → Edit App". Today that menu has been split into "Google Auth Platform → Branding / Audience / Data Access". Anyone following the doc literally lands on a different screen and gets stuck.

3. **They omit the gotchas that actually trip people.** None of these are mentioned anywhere:
   - Scope picker filters by enabled APIs — you have to enable Drive API *first* before drive scopes show up in the picker.
   - The "manually add scopes" textarea is the only way to add scopes that don't surface in the filtered list.
   - "External" + "Testing" mode caps you at 100 test users with the unverified-app warning.
   - The redirect URI for arawn's callback server can be either `http://localhost:<port>/callback` or `http://127.0.0.1:<port>/callback` depending on provider — Slack rejects one of those.
   - Drive defaults to full read+write scope, not `drive.readonly` — surprising for users who expect a "least privilege" default.
   - Google integration tokens persist across `/disconnect` if scopes change — users need to revoke at myaccount.google.com to fully reset.

There is also no **OAuth concept primer**. A new user looking at "OAuth 2.0 Desktop client + redirect URI + scopes + verification" with no context drowns. The hub doc should explain the model in plain English before linking out to per-provider walkthroughs.

The Drive UAT we just walked through together exercised every one of these gotchas. That conversation transcript is the source material for the initial doc draft.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- A new user with no OAuth experience can complete `/connect gmail` (or any integration) end-to-end by following our docs alone, without web search or trial and error.
- Every gotcha discovered in past UAT sessions is captured: stale UI screenshots, scope picker filtering, manual scope entry, "unverified app" warning, redirect URI host quirks, Drive's full-scope default, etc.
- Decision tree at the top: env var vs `arawn.toml` vs shared `[integrations.google]` block — three valid paths, when to pick which.
- Per-provider docs follow a consistent template: prerequisites → OAuth app creation → scopes → arawn config → connect flow → verification of success → troubleshooting.
- Common error messages map to specific causes ("invalid_redirect" → check the URI; "access_denied" → user clicked deny or app is unverified; "insufficient_scope" → re-add scopes and reconnect).
- Screenshots are dated and re-captured when provider UIs change. Dating is explicit ("UI as of 2026-05-06") so future readers know how stale they might be.

**Non-Goals:**
- The shared OAuth client / verification path — that's I-0037, gated on monetization decision.
- Server-side OAuth flow refactor — the runtime is fine; this is purely docs.
- Per-workspace Slack guidance — that's I-0034 territory.
- Video walkthroughs — start with text + screenshots; video maintenance is a tax we shouldn't pay yet.
- Translating to other languages — stay in English-only for now.

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

### Information architecture

```
docs/src/integrations/
├── README.md              ← hub: concepts + decision tree + provider matrix
├── oauth-primer.md        ← "what's OAuth, what's a scope, why this is annoying"
├── arawn-config.md        ← env var vs [integrations.<svc>] vs [integrations.google]
├── google.md              ← shared Google project setup (used by gmail/calendar/drive)
├── gmail.md               ← Gmail-specific scopes + tools
├── calendar.md            ← Calendar-specific scopes + tools
├── drive.md               ← Drive-specific scopes + tools (incl. full-scope default warning)
├── slack.md               ← Slack-specific app creation, scopes, dual-token model
├── atlassian.md           ← Atlassian 3LO walkthrough + cloud_id explanation
└── troubleshooting.md     ← error → cause → fix
```

The **hub README** is the entry point linked from the main docs landing page. It does:

1. **30-second framing**: "arawn integrations need an OAuth app. Today that means you create one. Here's why and what it costs you."
2. **Provider matrix** (recap of I-0037 table for current users): Google requires a Google Cloud project, Slack a Slack app, Atlassian a developer-console app. Time to set up: 10–20 minutes per provider.
3. **Decision tree**: do you want to share one Google OAuth app across Gmail + Calendar + Drive (recommended, use `[integrations.google]`), or per-service isolation (use `[integrations.gmail]` etc.)?
4. **Linkout to per-provider walkthroughs.**

### Per-provider walkthrough template

Each `gmail.md` / `calendar.md` / `drive.md` / `slack.md` / `atlassian.md` follows the same skeleton so users build a mental model after the first one:

```markdown
# {Service} integration

## What you get
- N tools land when configured: `{tool_1}`, `{tool_2}`, ...
- {1-paragraph description of what the agent can do with this}

## Prerequisites
- {OAuth app, account requirements, billing notes if any}

## Setup (≈ N minutes)

### 1. Enable the API in {provider console}
{Direct link with project_id placeholder + screenshot}

### 2. Add the OAuth scopes
{Exact scope strings, copy-paste block, screenshot of the Data Access page}
{If scopes don't appear in the picker → "use the manually add scopes textarea, paste these"}

### 3. Configure arawn
{Three options: env vars, [integrations.{svc}], [integrations.google] shared}
{TOML snippet to copy-paste}

### 4. Connect
{TUI command, what to expect in the browser, what success looks like}

## Verification
- `/integrations` should show {service} as connected with N capabilities.
- Try this prompt: "{minimal exercise prompt}"

## Troubleshooting
{Provider-specific issues, link to general troubleshooting.md}
```

### `oauth-primer.md` outline

For users who've never set up OAuth:

- "OAuth is how you let a third-party app (arawn) read your data without giving it your password."
- The four pieces: **client_id** (public app identifier), **client_secret** (proof you registered the app), **scope** (what permissions you're asking for), **redirect URI** (where to land after the user clicks Allow).
- Why every provider is different: each one has a different developer console, different scope vocabulary, different review process.
- Why "unverified app" warnings appear: providers want apps to be reviewed before they can request sensitive data from arbitrary users. Until verified, you can use your own app for yourself + ~100 test users.
- What arawn does with the tokens: stores them encrypted under `~/.arawn/tokens/`, refreshes silently, never sent off-device.

### Troubleshooting matrix

The `troubleshooting.md` doc is symptom-keyed:

| You see | Most likely cause | Fix |
|---|---|---|
| `Error 400: redirect_uri_mismatch` | Provider's allowed redirect doesn't match what arawn requested | Add `http://localhost:<port>/callback` (or `127.0.0.1` per provider) to the OAuth client's redirect URIs |
| `Error 403: access_denied` | Either you clicked deny in the consent screen OR the app's "test users" list doesn't include your account | Add yourself as a test user in the consent screen settings |
| `[integration] error: insufficient_scope` | You connected with one scope set, but the agent is trying to use a tool requiring a different one | `/disconnect <svc>`, add the missing scope in the consent screen, `/connect <svc>` again |
| `[integration] error: invalid_grant` | Stored refresh token expired or was revoked | `/disconnect <svc>` then `/connect <svc>` to re-auth |
| `Connection error: failed to reach <host>` | Network / DNS / API not enabled | Confirm the API is enabled in the provider console (e.g., Drive API requires explicit Library → Enable) |
| `[integration] connected: <svc>` but tools don't work | Likely a token-cache vs scope mismatch — common after adding new scopes | Revoke at `myaccount.google.com/permissions`, then `/connect` fresh |

### Screenshots policy

- Capture at consistent zoom/resolution (1.5x macOS default) so layout stays predictable.
- Crop tightly to the relevant control + parent menu so users can find it without seeing the whole console.
- Save under `docs/src/integrations/screenshots/{provider}/{step}.png`.
- Each image's caption includes the date: "(UI as of 2026-05-06)". When a provider UI changes, the caption is the audit trail.
- Don't screenshot anything secret — block out client_id, project name, etc.

### Discovery scaffolding for ongoing freshness

A short maintainer-facing doc at `docs/contributing/integration-docs.md`:

- "Run through this checklist after any provider UI change you notice."
- "When you UAT an integration, note any step that didn't match the docs and file a follow-up."
- "Re-screenshot the consent flow at least once per quarter."


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

**A. Patch the existing 3 docs in place.** Rejected. The structure is wrong — there's no hub, no shared concepts doc, and the per-provider docs duplicate a lot of preamble (env-var setup, OAuth flow explanation). A clean rewrite is faster than incremental patching once you account for the new providers (Drive, Atlassian) that have no doc at all.

**B. Auto-generate from code.** Rejected. We could auto-extract scope lists, tool names, and config keys from the integration crates. Worth doing for the *reference tables* (which scopes exist, which tools land), but the walkthrough prose — Cloud Console UI navigation, screenshots, troubleshooting recipes — is hand-written and won't auto-generate. Reserve auto-generation for a later pass.

**C. One giant page.** Rejected. The setup steps for each provider are long enough that a single page becomes a scroll-hostile wall. Per-provider pages with a shared template + cross-links is more maintainable.

**D. Defer until I-0037 (shared client) lands.** Rejected. I-0037 is gated on monetization and may take 6+ months. Users adopting arawn today need working docs now. The BYO docs we write here remain useful as the "advanced / privacy-conscious" path even after I-0037 ships.

## Implementation Plan **[REQUIRED]**

Phased so each task ships independently usable docs — readers can use the hub + Google walkthrough on day one, even before Slack/Atlassian docs are rewritten.

### Phase 1 — Hub + concept docs (T-TBD)
- `docs/src/integrations/README.md` — entry point with framing + decision tree.
- `docs/src/integrations/oauth-primer.md` — OAuth concepts in plain English.
- `docs/src/integrations/arawn-config.md` — env var vs TOML resolution explained.
- Update mdbook `SUMMARY.md` to surface the new structure.

**Acceptance:** a new user reading just the hub + primer can answer "what do I need before I start" without opening a per-provider doc.

### Phase 2 — Google walkthrough (T-TBD)
- `docs/src/integrations/google.md` — shared Google project setup, current UI navigation (Google Auth Platform), API enable flow, scope picker quirks, manual scope entry.
- Per-service docs: `gmail.md`, `calendar.md`, `drive.md` — thin wrappers on top of `google.md` covering tool list + scope-specific notes (Drive's full-scope default warning, Gmail's three-scope split, Calendar's events-only scope).
- Screenshots dated 2026-05-06.

**Acceptance:** UAT a fresh `/connect gmail` (or drive/calendar) on a clean macOS account using only the docs. No web search permitted during UAT.

### Phase 3 — Slack walkthrough (T-TBD)
- `docs/src/integrations/slack.md` — full rewrite covering Slack's app creation, the dual-token model (bot + user), the redirect URI host quirk (`localhost`, not `127.0.0.1`), the fixed-port mode, and the precise scope list including the workspace-admin caveat.

**Acceptance:** UAT a fresh `/connect slack` using only the docs.

### Phase 4 — Atlassian walkthrough (T-TBD)
- `docs/src/integrations/atlassian.md` — covering 3LO setup, accessible-resources / cloud_id discovery, scope additions for Jira + Confluence, tool list.

**Acceptance:** UAT a fresh `/connect atlassian` using only the docs.

### Phase 5 — Troubleshooting + maintainer guide (T-TBD)
- `docs/src/integrations/troubleshooting.md` — symptom-keyed error matrix.
- `docs/contributing/integration-docs.md` — how/when to update screenshots and gotchas.

**Acceptance:** for each known error message in the engine logs and TUI, a row exists in the matrix with cause + fix.

### Sequencing

1 → 2 are sequential (concept docs unblock the per-provider page format). 3 and 4 can run in parallel after 2. 5 lands last and pulls in the troubleshooting items discovered during phases 2–4.

### Open questions to resolve before decomposition

1. **Where does the hub README link from?** Top-level `docs/src/SUMMARY.md`, the main project README, both?
2. **Screenshot storage strategy.** Inline in the markdown? CDN? GitHub Pages Pages-Native? Must work for both local mdbook build and the published site.
3. **Versioning.** Do we tag docs with arawn binary version (e.g. "tested against arawn 0.4.x")? Helps users on older binaries know which docs apply.
4. **Localization stub.** Do we set up i18n scaffolding now even though we're English-only, or defer until someone asks?
5. **Tone.** "You" / second-person walkthroughs (current draft assumes this), or imperative? Standardize once before writing.