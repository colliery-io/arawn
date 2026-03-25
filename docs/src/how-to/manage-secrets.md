# Manage Secrets

This guide covers storing, retrieving, and rotating API keys and other secrets in Arawn. You will learn the full resolution chain, how to use the encrypted secret store, and how to reference secrets safely in tool parameters.

## Prerequisites

- Arawn installed and initialized (`arawn config init`)
- A terminal with access to the `arawn` CLI

## Understand the resolution chain

When Arawn needs an API key for an LLM backend, it checks four sources in order. The first match wins:

| Priority | Source | Description |
|----------|--------|-------------|
| 1 (highest) | Age-encrypted store | `~/.config/arawn/secrets.age` |
| 2 | System keyring (legacy) | OS keychain via the `keyring` crate |
| 3 | Environment variable | Backend-specific env var |
| 4 (lowest) | Config file | Plaintext in `config.toml` (not recommended) |

No prefix syntax is needed. Arawn walks the chain silently and uses the first value it finds. If a key is loaded from the config file, Arawn logs a warning.

## Store an API key for a backend

Use `arawn config set-secret` to store a key in the age-encrypted store. The command prompts for the value interactively so it never appears in your shell history:

```bash
arawn config set-secret anthropic
# Enter value for 'anthropic_api_key' (input hidden): ****
# Secret 'anthropic_api_key' stored in encrypted store.
```

Supported backend names: `anthropic`, `openai`, `groq`, `ollama`, `custom`, `claude-oauth`.

The key name is derived automatically from the backend (e.g., `anthropic` becomes `anthropic_api_key`). You do not choose the name.

## Delete a backend API key

```bash
arawn config delete-secret groq
# Secret 'groq_api_key' deleted.
```

This removes the key from the age-encrypted store only. It does not affect environment variables or keyring entries.

## Store a general-purpose secret

For secrets that are not backend API keys (e.g., a GitHub token used in tool parameters), use the `arawn secrets` commands:

```bash
# Store — prompts for value, input is hidden
arawn secrets set github_token
# Enter value for 'github_token' (input hidden): ****
# Secret 'github_token' stored in encrypted store.
# Use ${{secrets.github_token}} in tool parameters to reference it.

# List stored secret names (never shows values)
arawn secrets list
# Stored secrets:
#   anthropic_api_key
#   github_token
#   groq_api_key
# 3 secret(s) total.

# Delete
arawn secrets delete github_token
# Secret 'github_token' deleted.
```

## Reference secrets in tool parameters

Secrets can be injected into tool parameters at execution time using the `${{secrets.<name>}}` syntax:

```json
{
  "tool": "web_fetch",
  "parameters": {
    "url": "https://api.github.com/repos/owner/repo",
    "headers": {
      "Authorization": "Bearer ${{secrets.github_token}}"
    }
  }
}
```

The resolution happens just before the tool executes. The actual secret value is never logged, never included in LLM context, and never written to session history.

## Encryption files

Arawn uses [age](https://age-encryption.org/) (X25519) encryption. Two files are involved:

| File | Purpose |
|------|---------|
| `~/.config/arawn/identity.age` | Your private key (generated automatically on first use) |
| `~/.config/arawn/secrets.age` | Encrypted JSON map of all secrets |

Both files are created with `0o600` permissions (owner read/write only). Arawn logs a warning on startup if these files are group- or world-readable.

> For details on the encryption internals, atomic write patterns, and the full
> security model, see [Security Model](../explanation/security-model.md).

## Use environment variables as fallback

If you prefer not to use the encrypted store (e.g., in CI/CD pipelines or containers), set the appropriate environment variable:

| Backend | Environment Variable |
|---------|---------------------|
| Anthropic | `ANTHROPIC_API_KEY` |
| OpenAI | `OPENAI_API_KEY` |
| Groq | `GROQ_API_KEY` |
| Ollama | `OLLAMA_API_KEY` |
| Custom | `LLM_API_KEY` |
| Claude OAuth | `ANTHROPIC_API_KEY` |

```bash
export ANTHROPIC_API_KEY="sk-ant-api03-..."
export GROQ_API_KEY="gsk_..."
```

Environment variables take priority 3 in the resolution chain, so they are used only when the age store and keyring have no entry for that backend.

## Handle OAuth tokens

OAuth tokens (used with Claude OAuth / Anthropic MAX) are stored separately in `~/.config/arawn/oauth-tokens.json`. This file:

- Uses the same atomic write-to-temp-then-rename pattern as `secrets.age`.
- Has `0o600` permissions enforced at write time.
- Contains both access and refresh tokens.
- Is managed automatically by `arawn auth login`; you do not edit it manually.

## Rotate a secret

To rotate a key, store the new value under the same name. The old value is overwritten:

```bash
# Rotate your Anthropic key
arawn config set-secret anthropic
# Enter value for 'anthropic_api_key' (input hidden): ****

# Or rotate a general secret
arawn secrets set github_token
# Enter value for 'github_token' (input hidden): ****
```

No restart is required if Arawn is running as a server. The secret store is re-read on demand.

## Migrate from keyring to age store

If you previously stored keys in the OS keyring, those entries still work (priority 2). To migrate to the age store (priority 1):

1. Note which keys are in the keyring:

   ```bash
   # macOS
   security find-generic-password -s arawn

   # Linux
   secret-tool search service arawn
   ```

2. Store each key in the age store:

   ```bash
   arawn config set-secret anthropic
   arawn config set-secret groq
   ```

3. Optionally remove old keyring entries:

   ```bash
   # macOS
   security delete-generic-password -s arawn -a anthropic_api_key

   # Linux
   secret-tool clear service arawn username anthropic_api_key
   ```

## Best practices

- **Prefer `set-secret` or environment variables** over plaintext `api_key` in `config.toml`.
- **Never commit secrets to version control.** Add `config.toml` to `.gitignore` if it contains keys.
- **Use `${{secrets.name}}` in tool parameters** instead of hardcoding tokens.
- **Restrict file permissions.** Arawn sets `0o600` automatically, but verify with `ls -la ~/.config/arawn/`.
- **Back up `identity.age` securely.** If you lose this file, you cannot decrypt `secrets.age`. There is no recovery mechanism.
