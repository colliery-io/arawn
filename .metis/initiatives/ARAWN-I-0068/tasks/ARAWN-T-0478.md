---
id: p2-2-feed-auth-lifecycle-auto
level: task
title: "P2-2: Feed auth lifecycle — auto-pause on AuthExpired, reconnect-needed state, token-persist hardening"
short_code: "ARAWN-T-0478"
created_at: 2026-06-12T12:02:12.117714+00:00
updated_at: 2026-06-12T12:02:12.117714+00:00
parent: ARAWN-I-0068
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0068
---

# P2-2: Feed auth lifecycle — auto-pause on AuthExpired, reconnect-needed state, token-persist hardening

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0068]] — implements P2-2 (CRITICAL). Surfaces its "reconnect needed" state via the `/status` surface ([[ARAWN-T-0476]]).

## Objective **[REQUIRED]**

Make feeds survive auth failure gracefully: auto-pause a feed on `AuthExpired` with a persisted "reconnect needed" reason instead of letting cloacina retry forever and burn API quota, distinguish "reconnect" from a transient blip, and harden token-persist failures.

**The defect:** `FeedError::Auth` becomes a generic `TaskError::ExecutionFailed`; cloacina retries indefinitely (`arawn-feeds/src/dispatch.rs:93-97, 168-182`) — days of failed runs, discoverable only by manually reading feed status. "Token revoked, reconnect needed" vs "network blip" are indistinguishable. A refreshed token that fails to persist is just a log warning; the in-memory token works until restart, then the stale token reloads and fails confusingly (`arawn-integrations/src/google_common.rs:137-143`, `arawn-auth/src/oauth2.rs:178-180`).

### Type
- [x] Bug / Feature — feed reliability for unattended operation

### Priority
- [x] P1 - High (CRITICAL finding; "feeds run unattended for weeks" depends on it)

## Acceptance Criteria **[REQUIRED]**

- [ ] An `AuthExpired`/revoked-token failure auto-pauses the feed (clears `enabled` / sets a paused state) with a persisted, user-visible "reconnect needed" reason — no indefinite retry spam.
- [ ] Auth errors map to a NON-retryable task outcome (distinct from transient network errors, which still retry).
- [ ] A refreshed token that fails to persist marks the integration degraded (surfaced), not a silent log warning that strands a stale token on restart.
- [ ] The paused state + reason are visible via `/status` ([[ARAWN-T-0476]]).
- [ ] Tests: an injected `AuthExpired` pauses the feed with the reason; a transient error does not.
- [ ] `angreal check all` and `angreal test unit` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
In `arawn-feeds` dispatch, branch on `FeedError::Auth` → pause the feed row + write a "reconnect needed" reason (read by `/status`), and return a non-retryable task outcome to cloacina. Thread `AuthError::AuthExpired` distinctly through the feed layer. Make the token-persist failure path return an error that flags the integration degraded.

### Dependencies
Surfaced via [[ARAWN-T-0476]] (status). Touches `arawn-feeds/dispatch.rs`, `arawn-auth/oauth2.rs`, `arawn-integrations/google_common.rs`.

### Risk Considerations
Don't pause on transient auth blips — only on genuine `AuthExpired`/revocation. The pause must be reversible (user reconnects → feed resumes). Quota-burn is the cost of getting this wrong.

## Status Updates **[REQUIRED]**

*To be added during implementation*