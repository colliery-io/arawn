---
id: add-rate-limiting-end-to-end-tests
level: task
title: "Add rate limiting end-to-end tests"
short_code: "ARAWN-T-0297"
created_at: 2026-03-08T20:21:19.463187+00:00
updated_at: 2026-03-08T20:21:19.463187+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Add rate limiting end-to-end tests

## Objective

Rate limiter has unit tests but no test verifies rate limiting through the actual HTTP request path. Add integration tests that send requests through the server and verify 429 responses when limits are exceeded.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P2 - Medium (nice to have)

### Technical Debt Impact
- **Current Problems**: Rate limiter unit tests pass but the middleware integration is untested. Misconfigured middleware could silently disable rate limiting.
- **Benefits of Fixing**: Confidence that rate limiting actually works in production request flow.
- **Risk Assessment**: Low-medium — rate limiting is a safety/abuse-prevention mechanism.

## Acceptance Criteria

- [ ] Test that requests within the limit succeed (200)
- [ ] Test that requests exceeding the limit return 429
- [ ] Test that rate limits reset after the window expires
- [ ] Test per-session rate limiting (one session's limits don't affect another)
- [ ] `cargo test -p arawn-server` passes
- [ ] `angreal check clippy` clean

## Implementation Notes

### Technical Approach
- Spin up test server with low rate limits (e.g., 3 requests per second)
- Send rapid requests via the HTTP client
- Assert on status codes (200 vs 429)
- Use `tokio::time::sleep` to verify window reset

### Files
- `crates/arawn-server/tests/` (new integration test)

## Status Updates

*To be added during implementation*