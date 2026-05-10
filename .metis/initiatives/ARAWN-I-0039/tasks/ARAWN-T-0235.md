---
id: atlassian-token-refresh-drops
level: task
title: "Atlassian token refresh drops sites extras (and silent-fail on first connect)"
short_code: "ARAWN-T-0235"
created_at: 2026-05-10T00:00:00+00:00
updated_at: 2026-05-10T00:00:00+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
---

# Atlassian token refresh drops sites extras (and silent-fail on first connect)

## Parent Initiative

[[ARAWN-I-0039]]

## Severity

P1 — found during T-0218 UAT after the second `/connect atlassian` cycle. Atlassian feeds (jira/*, confluence/space-archive) consistently break ~1 hour after a successful connect, with the same `auth: atlassian (no accessible sites — reconnect)` error. User has to reconnect on every session.

## Two related bugs in the atlassian integration

### Bug A: silent-fail on accessible-resources discovery during `connect()`

In `crates/arawn-integrations/src/atlassian/integration.rs::connect`, after the OAuth code exchange completes, we call `fetch_accessible_resources` to populate `token.extras["sites"]`. The `Err(e)` arm at line 302 just logs a warn and **persists the token without sites**:

```rust
match fetch_accessible_resources(&outcome.token.access).await {
    Ok(sites) => { ... persist token with sites ... }
    Err(e) => {
        tracing::warn!(error = %e, "atlassian accessible-resources discovery failed");
        ctx.publish_progress(...).await;
        // ← token already persisted by run_oauth_flow without sites
    }
}
```

The first `feed_run` then fails with "no accessible sites — reconnect", but the user just connected. They reconnect, this time discovery succeeds, sites populate. UX is "connect twice, second one sticks." Observed twice during UAT.

### Bug B: refresh path drops sites extras

This is the bigger problem. Atlassian access tokens expire after ~1 hour. Token refresh happens transparently in `AtlassianClient::fresh_access_token`. The new token is persisted via `save_token`, but `extras["sites"]` is **not carried forward**. Result: every refresh wipes the sites list, and the next call fails with "no accessible sites".

Reproduction during UAT:
- 01:25 UTC: `/connect atlassian` succeeds, sites populated, all 3 atlassian feeds work.
- 02:25 UTC: token expires. Background refresh fires.
- 03:11 UTC: `feed_register` triggers backfill spawn → `feed_run_force` → "no accessible sites — reconnect".

Confirmed by checking the persisted token's `extras` is empty after the 1-hour mark.

## Suggested fix

### Bug A

Don't persist the token in `run_oauth_flow` until accessible-resources discovery has completed. Either:
- Move discovery inside the OAuth flow (couples discovery to the flow but eliminates the silent-fail).
- After `run_oauth_flow` returns, attempt discovery in a retry loop (3 attempts with backoff). On final failure, surface as `IntegrationError::NotConnected` and let the user retry `/connect`.

Whichever route, the persisted token should ALWAYS have `extras["sites"]` populated, or no token should be persisted.

### Bug B

In the refresh path (likely in `arawn-auth` or wherever `fresh_access_token` lives for atlassian), preserve the prior token's `extras` when writing the new token. Pseudo-code:

```rust
async fn fresh_access_token(&self) -> Result<String, ...> {
    let prior = self.load_token()?;
    if prior.is_still_valid() { return Ok(prior.access); }
    let mut new_token = oauth_refresh(&prior.refresh).await?;
    // Preserve the prior token's extras (sites, granted_scope_set, etc.)
    new_token.extras = prior.extras;
    self.save_token(&new_token)?;
    Ok(new_token.access)
}
```

Alternative (more correct but more work): re-run `fetch_accessible_resources` on every refresh. Costs an extra HTTP call per refresh but stays in sync if the user added/removed sites in Atlassian's admin UI.

## Acceptance Criteria

- [ ] Bug A: `connect()` either retries `fetch_accessible_resources` or refuses to persist the token if discovery fails. After a successful `/connect atlassian`, the persisted token always has non-empty `extras["sites"]`.
- [ ] Bug B: token refresh preserves `extras["sites"]` (and any other extras keys we set later). Add a unit test that simulates a refresh and asserts extras carry over.
- [ ] Smoke test in UAT: `/connect atlassian`, register a feed, wait 70+ minutes (past the 1-hour token expiry), trigger another `feed_run` → still works.
- [ ] `angreal check workspace` and `angreal check clippy` clean.

## Workaround for the user (current)

`/disconnect atlassian` + `/connect atlassian` whenever the "no accessible sites" error appears. Lasts ~1 hour.

## Related

- T-0229 — Confluence scope mismatch (closed). The user noticed this bug while trying to test the scope fix.
- T-0227 / T-0233 — backfill spawn loops. Bug B is what made the Jira `since=` smoke test fail post-refresh.

## Status Updates

*To be added during implementation*
