---
id: security-hardening
level: initiative
title: "Security Hardening"
short_code: "ARAWN-I-0029"
created_at: 2026-03-22T00:39:09.435413+00:00
updated_at: 2026-03-22T15:46:14.084987+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: S
initiative_id: security-hardening
---

# Security Hardening Initiative

## Context

Comprehensive security audit of Arawn (March 2026) identified 4 critical and 5 high-severity security issues. As Arawn moves from development to daily personal use, these must be addressed — especially since the system executes shell commands, writes files, and makes network requests on behalf of an AI agent.

## Goals & Non-Goals

**Goals:**
- Fix all critical security vulnerabilities (SSRF, token storage, header spoofing, path validation)
- Fix all high-severity issues (CORS, rate limit bypass, command validator, WebSocket hardening)
- Establish security controls that are missing entirely (CSRF, audit logging, secret rotation)

**Non-Goals:**
- Full penetration testing (out of scope for this initiative)
- Compliance certification
- Multi-user access control beyond current auth model

## Detailed Design

### Critical Fixes
1. **SSRF protection in WebFetchTool** (`arawn-agent/src/tools/web.rs:262`): Add DNS resolution validation, reject private/loopback/link-local/cloud-metadata addresses after hostname resolution
2. **OAuth token encryption** (`arawn-oauth/src/token_manager.rs:126`): Set 0o600 permissions on `oauth-tokens.json`; migrate to age-encrypted secret store
3. **Tailscale header verification** (`arawn-server/src/auth.rs:185`): Verify requests come from Tailscale interface; document that server MUST bind to Tailscale interface when using this auth mode
4. **Web download path validation** (`arawn-agent/src/tools/web.rs:352`): Apply `reject_traversal` and FsGate validation to download paths; add `web_fetch` to GATED_TOOLS

### High Fixes
5. **CORS lockdown on OAuth proxy** (`arawn-oauth/src/proxy.rs:90`): Restrict to `http://127.0.0.1:*` and `http://localhost:*`
6. **Rate limit proxy trust** (`arawn-server/src/ratelimit.rs:79`): Add `trust_proxy: bool` config; only read X-Forwarded-For when enabled
7. **Command validator hardening** (`arawn-agent/src/tool/command_validator.rs:89`): Parse commands more robustly before checking blocklist
8. **WebSocket rate limiting** (`arawn-server/src/lib.rs:83`): Add dedicated connection rate limiter; implement origin validation using `ws_allowed_origins`
9. **WebSocket message size enforcement**: Pass `max_ws_message_size` config to WebSocket upgrade

### Missing Controls
10. **Sandbox deny-read for Arawn's own secrets**: Add `~/.config/arawn/secrets.age`, `identity.age`, `oauth-tokens.json` to deny-read paths
11. **Auth-disabled warning**: Log warning when auth is disabled and bind address is not localhost

## Implementation Plan

- Phase 1 (P0): Critical fixes 1-4 (1-2 days)
- Phase 2 (P1): High fixes 5-9 (2-3 days)
- Phase 3 (P2): Missing controls 10-11 (1 day)