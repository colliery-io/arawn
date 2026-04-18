---
id: oauth2-pkce-flow-with-refresh-and
level: task
title: "OAuth2 PKCE flow with refresh and local callback HTTP listener"
short_code: "ARAWN-T-0180"
created_at: 2026-04-17T03:01:14.682872+00:00
updated_at: 2026-04-17T03:12:01.378817+00:00
parent: ARAWN-I-0029
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0029
---

# OAuth2 PKCE flow with refresh and local callback HTTP listener

## Parent Initiative

[[ARAWN-I-0029]]

## Objective

Implement a generic OAuth2 PKCE flow that any provider (Google, Slack, etc.) can drive. Includes the local HTTP callback listener that the user's browser redirects to after consent.

Lives in `crates/arawn-integration/src/auth/`:
- `oauth2.rs` — PKCE code generation, authorization URL construction, code → token exchange, refresh.
- `server.rs` — local listener bound to `127.0.0.1:0` (resolved port passed back into redirect URL), waits for one callback, returns the auth code and `state`.
- `provider_config.rs` — generic `OAuthProviderConfig { auth_url, token_url, client_id, client_secret, scopes, redirect_path }` so providers don't reimplement the protocol.

Token storage and arawn.toml config are out of scope (T-0181, T-0183). This task produces an `OAuthClient` that returns a `Token { access, refresh, expires_at }` struct in memory.

Estimated size: **M** (2–3 days).

### Priority
- [x] P2 - Medium (gates everything that needs OAuth)

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement  
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `OAuthClient::new(OAuthProviderConfig)` constructor
- [ ] `OAuthClient::start_flow() -> (auth_url: Url, csrf_state: String, pkce_verifier: String)` — generates code_verifier, code_challenge, state
- [ ] `OAuthClient::exchange_code(code, csrf_state, pkce_verifier) -> Result<Token, IntegrationError>` — token exchange
- [ ] `OAuthClient::refresh(refresh_token) -> Result<Token, IntegrationError>` — refresh flow
- [ ] `CallbackServer::listen() -> Result<CallbackResult, IntegrationError>` — binds 127.0.0.1:0, accepts one redirect, returns `(code, state)`
- [ ] Resolved redirect URL (`http://127.0.0.1:<port>/<redirect_path>`) is exposed by the server before `listen()` returns, so callers can pass it into `OAuthClient::authorize_url(...)`
- [ ] CSRF state is verified — mismatched state returns `IntegrationError::InvalidConfig`
- [ ] PKCE: code_challenge is `S256` of code_verifier
- [ ] Mock-server tests using a tiny `wiremock`/`mockito`-style harness covering: happy path (start → callback → exchange), expired refresh (returns `AuthExpired`), provider returning HTTP error (returns `ApiError`), state mismatch
- [ ] Integration test: spawn `CallbackServer`, simulate browser GET, assert the returned code+state matches
- [ ] Depends on ARAWN-T-0179 (uses `IntegrationError`)

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates

- New module `arawn-integration::auth` with `oauth2.rs` and `server.rs`. Skipped a separate `provider_config.rs` — `OAuthProviderConfig` is small enough to live alongside `OAuthClient` in `oauth2.rs`.
- Implemented from scratch (no `oauth2` crate dep) on top of `reqwest` + `sha2` + `base64` + `url` + `rand`. PKCE S256 verified against the RFC 7636 Appendix B test vector.
- `OAuthClient`: `start_flow(&redirect)` → `AuthRequest { authorization_url, csrf_state, pkce_verifier }`; `exchange_code(code, &redirect, verifier)` and `refresh(refresh_token)`. Refresh preserves the prior refresh token if the provider omits one (Google's behaviour).
- `Token { access, refresh: Option, expires_at: Option, scope, token_type }` with `is_expired()`. Auth params include `access_type=offline` and `prompt=consent` so Google returns refresh tokens.
- `CallbackServer::bind("/callback")` binds 127.0.0.1:0 and exposes the resolved redirect URL via `redirect_uri()` before listening. `listen_with_timeout(d)` accepts one HTTP request, parses `?code/?state/?error`, always responds with a small success page (so the user's browser doesn't hang on errors), then surfaces error details to the caller.
- 13 unit tests including: PKCE RFC vector, start_flow URL contents, callback happy path, missing-code, provider-error, timeout, token-exchange (decoded against an in-process HTTP stub), refresh→AuthExpired on 400, refresh preserves refresh token when provider omits it.
- **Spec deviation**: CSRF state matching is the *caller's* responsibility — `CallbackServer` returns the raw `(code, state)` pair, then `OAuthClient::exchange_code` is invoked by code that has the original state in scope. Putting state-checking inside `OAuthClient` would force passing it through three boundaries for no benefit. The CLI task (T-0184) will compare states.
- All `cargo test -p arawn-integration` tests pass (13/13).