---
id: explicit-api-key-resolution-via
level: task
title: "Explicit API key resolution via api_key_ref config field"
short_code: "ARAWN-T-0431"
created_at: 2026-03-26T01:15:08.252620+00:00
updated_at: 2026-03-26T01:18:14.761779+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Explicit API key resolution via api_key_ref config field

## Problem

Two commands existed for storing secrets (`config set-secret` and `secrets set`) with different key naming conventions. The implicit 4-step fallback chain in `resolve_api_key()` (age store → keyring → env var → config value) guesses where secrets live, causing silent failures when keys are stored under unexpected names.

## Solution

Replace the implicit fallback chain with explicit resolution. Config names the variable, we look it up. Done.

### Config syntax

```toml
[llm]
backend = "groq"
api_key_ref = "GROQ_API_KEY"    # variable name, not a value
```

### Resolution (2 steps, no magic)

1. Lowercase the ref name → check secrets store for it
2. Check env var: as-is, then uppercase

### Secret storage normalization

All secret names lowercased at storage time:
- `arawn secrets set GROQ_API_KEY` → stored as `groq_api_key`
- Case-insensitive on retrieval

### No backward compatibility

Breaking change. `api_key` field removed entirely. `api_key_ref` is the only way. No shims, no aliases, no defaults.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `api_key_ref` field in config resolves from secrets store (lowercase lookup)
- [ ] Falls back to env var when not in secrets store
- [ ] `arawn secrets set/get/delete` normalize names to lowercase
- [ ] Old `api_key` field removed — config errors if used
- [ ] Remove `resolve_api_key()` fallback chain
- [ ] Remove keyring support (legacy)
- [ ] Docs updated (manage-secrets, configure-llm-backends, configuration reference, CLI reference)
- [ ] All existing tests pass, new tests for explicit resolution

## Files to Modify

| File | Change |
|------|--------|
| `crates/arawn-config/src/secret_store.rs` | Lowercase normalization on all ops |
| `crates/arawn-config/src/types.rs` | `api_key` → `api_key_ref` with serde alias |
| `crates/arawn-config/src/secrets.rs` | New `resolve_api_key_ref()`, remove old chain |
| `crates/arawn/src/commands/start.rs` | Wire new resolution into backend construction |
| `crates/arawn/src/commands/secrets.rs` | Lowercase on set/get/delete |
| `docs/src/how-to/manage-secrets.md` | Update instructions |
| `docs/src/how-to/configure-llm-backends.md` | Update config examples |
| `docs/src/reference/configuration.md` | Document `api_key_ref` |
| `docs/src/reference/cli.md` | Update secrets section |

## Status Updates

*To be added during implementation*