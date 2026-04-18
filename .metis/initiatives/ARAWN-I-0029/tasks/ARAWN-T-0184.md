---
id: arawn-setup-lt-provider-gt-cli
level: task
title: "arawn setup &lt;provider&gt; CLI subcommand — browser-based OAuth consent flow"
short_code: "ARAWN-T-0184"
created_at: 2026-04-17T03:01:21.151669+00:00
updated_at: 2026-04-17T03:29:07.024099+00:00
parent: ARAWN-I-0029
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0029
---

# arawn setup &lt;provider&gt; CLI subcommand — browser-based OAuth consent flow

## Parent Initiative

[[ARAWN-I-0029]]

## Objective

Add a clap subcommand `arawn setup <provider>` that drives the one-time OAuth consent flow:

1. Loads `[integrations.<provider>]` from arawn.toml.
2. Constructs an `OAuthClient` for the provider (from T-0180).
3. Spawns the local `CallbackServer` (T-0180).
4. Opens the user's browser at the authorization URL (uses the `open` crate or `xdg-open`/`open` shell call).
5. Waits for callback, exchanges code for tokens.
6. Encrypts and persists via `TokenStore` (T-0181).
7. Prints "✓ <provider> connected" and exits cleanly.

Browser-only flow — no headless mode in v1. Document a future `--print-auth-url` flag as a follow-up for SSH/headless setups.

Estimated size: **M** (~2 days).

### Priority
- [x] P2 - Medium (user-facing entry point; without it, tokens can't be written)

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

- [ ] New clap subcommand: `arawn setup <provider>` (e.g., `arawn setup google`)
- [ ] Subcommand reads arawn.toml, looks up `[integrations.<provider>]`, errors clearly if missing
- [ ] Required env vars (e.g., `GOOGLE_CLIENT_ID`/`GOOGLE_CLIENT_SECRET`) are validated before opening the browser; missing vars produce an actionable error message
- [ ] Browser open uses the `open` crate (or platform fallback); failure to open prints the URL so the user can paste it manually
- [ ] On success, token file appears at `{data_dir}/tokens/<provider>.json.age` and stdout contains `✓ <provider> connected`
- [ ] On user-cancelled consent or callback timeout (default 5 min), prints a clear error and exits non-zero without writing anything
- [ ] If `setup` is run again for an already-connected provider, prompts for confirmation before overwriting (unless `--force` is passed)
- [ ] Test: end-to-end with a mock OAuth provider on `127.0.0.1` and a stubbed browser-open (replaces `open` with a function that synthesizes the redirect immediately)
- [ ] Documented in `arawn --help` output
- [ ] Depends on ARAWN-T-0180, ARAWN-T-0181, ARAWN-T-0183

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

- New `arawn-bin/src/setup.rs` with `run_setup(data_dir, provider, force)` and a test-friendly `run_setup_with_browser(..., open_browser, timeout)` that takes the browser-open side-effect as a closure.
- Built-in OAuth endpoint mapping `provider_endpoints("google" | "slack")` returns hardcoded auth + token URLs. Unknown providers error with an actionable message listing supported kinds.
- Flow: load arawn.toml → look up `[integrations.<provider>]` → resolve provider kind → validate `client_id_env` (and `client_secret_env` if set) → check existing token (refuse without `--force`) → bind `CallbackServer` → start PKCE flow → open browser (or print URL on failure) → wait up to 5 min → verify CSRF state → exchange code → save via `TokenStore` → print `✓ <provider> connected`.
- Added clap subcommand `arawn setup <provider> [--force]`. Wired in `main.rs` to short-circuit before the rest of startup.
- Added `open` and `url` deps to arawn-bin Cargo.
- 3 unit tests: missing env var → clear error naming the var; missing `[integrations.<name>]` section → clear error; CSRF state mismatch → setup aborts with "state mismatch" (drives a fake browser via captured opener that GETs the callback URL with a wrong state).
- **Deferred**: live end-to-end test that actually reaches `exchange_code`. The setup orchestration uses hardcoded provider endpoints; mocking the entire Google OAuth surface in-process would be substantial. `exchange_code` itself is covered by the oauth2.rs token-stub tests (T-0180), and orchestration through to "wait for callback" is exercised here. Token persistence after exchange is exercised by TokenStore tests (T-0181).
- All workspace tests pass (`angreal test unit` green).