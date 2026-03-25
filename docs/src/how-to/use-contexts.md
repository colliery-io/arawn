# Use Contexts for Multi-Server

This guide explains how to use contexts to manage connections to multiple Arawn servers. Contexts are named connection profiles -- similar to kubeconfig contexts in Kubernetes -- that bundle a server URL, authentication credentials, and default settings into a single switchable name.

## What contexts are

A context is a named entry in the client configuration that stores:

- **Server URL** -- where the Arawn server is listening.
- **Authentication** -- how to authenticate with that server (API key, OAuth, bearer token, or none).
- **Default workstream** -- which workstream to use when none is specified.
- **Timeout** -- connection timeout override for slow links.

Contexts let you run commands against different servers without repeatedly passing `--server` and credential flags.

## Configuration file

Contexts are stored in:

```
~/.config/arawn/client.yaml
```

The file format follows a kubeconfig-style structure:

```yaml
api-version: v1
kind: ClientConfig

current-context: local

contexts:
  - name: local
    server: http://localhost:8080

  - name: staging
    server: https://arawn-staging.example.com:8443
    auth:
      type: api-key
      key-file: ~/.config/arawn/keys/staging.key
    workstream: testing
    timeout: 60

  - name: prod
    server: https://arawn.example.com
    auth:
      type: bearer
      token-env: ARAWN_PROD_TOKEN
    workstream: default

defaults:
  timeout: 30
  workstream: default
```

The `defaults` section provides fallback values applied when a context does not specify its own timeout or workstream.

## Create a context

Use `arawn config set-context` to create or update a context:

```sh
arawn config set-context dev --server http://localhost:8080
```

Add a default workstream and timeout:

```sh
arawn config set-context dev \
  --server http://localhost:8080 \
  --workstream scratch \
  --timeout 15
```

To update an existing context, run `set-context` with the same name. Only the flags you pass are changed; unspecified fields are preserved.

```sh
# Change just the server URL for an existing context
arawn config set-context dev --server http://localhost:9090
```

## Switch contexts

Set the active context:

```sh
arawn config use-context prod
```

All subsequent commands use the `prod` context's server URL and credentials until you switch again.

## List contexts

```sh
arawn config get-contexts
```

Output shows all configured contexts with the current one marked:

```
  CURRENT   NAME      SERVER
  *         local     http://localhost:8080
            staging   https://arawn-staging.example.com:8443
            prod      https://arawn.example.com
```

## Show the current context

```sh
arawn config current-context
```

```
local
```

## Delete a context

```sh
arawn config delete-context old
```

If you delete the current context, the `current-context` field is cleared. You will need to switch to another context or pass `--server` explicitly.

## Authentication types

Each context can specify one of four authentication types.

### None (default)

No authentication is sent. Use this for local development servers running without a token.

```yaml
contexts:
  - name: local
    server: http://localhost:8080
    auth:
      type: none
```

Or simply omit the `auth` field entirely.

### API key

Read an API key from a file or environment variable:

```yaml
contexts:
  - name: home
    server: https://arawn.home.lan:8443
    auth:
      type: api-key
      key-file: ~/.config/arawn/keys/home.key
```

```yaml
contexts:
  - name: home
    server: https://arawn.home.lan:8443
    auth:
      type: api-key
      key-env: ARAWN_HOME_KEY
```

When both `key-file` and `key-env` are set, the file is tried first. If the file does not exist, the environment variable is used as a fallback.

### OAuth

Authenticate via an OAuth 2.0 flow:

```yaml
contexts:
  - name: work
    server: https://arawn.company.com
    auth:
      type: oauth
      client-id: arawn-tui
      token-file: ~/.config/arawn/tokens/work.json
```

OAuth tokens are managed separately by the OAuth flow (`arawn auth login`). The `token-file` is a cache for the most recent token.

### Bearer token

Pass a pre-shared bearer token from a file or environment variable:

```yaml
contexts:
  - name: prod
    server: https://arawn.example.com
    auth:
      type: bearer
      token-file: ~/.config/arawn/keys/prod.token
```

```yaml
contexts:
  - name: prod
    server: https://arawn.example.com
    auth:
      type: bearer
      token-env: ARAWN_PROD_TOKEN
```

Like API key auth, `token-file` is tried before `token-env`.

## Override the context per command

Use the global `--context` flag to target a specific context without switching:

```sh
arawn --context prod status
arawn --context staging ask "What is the current memory usage?"
arawn --context dev chat
```

This does not change the current context in `client.yaml`.

## Server URL resolution order

When Arawn needs to determine which server to talk to, it checks these sources in order:

| Priority | Source | Example |
|----------|--------|---------|
| 1 | `--server` flag | `arawn --server http://host:8080 status` |
| 2 | `--context` flag | `arawn --context prod status` |
| 3 | Current context | Set via `arawn config use-context` |
| 4 | `ARAWN_SERVER_URL` env | `export ARAWN_SERVER_URL=http://host:8080` |
| 5 | Default | `http://localhost:8080` |

The first source that provides a value wins. This means `--server` always takes precedence, and if nothing else is configured, Arawn falls back to localhost.

## Per-context defaults

Each context can override the default workstream and connection timeout:

```yaml
contexts:
  - name: research
    server: https://arawn.home.lan:8443
    workstream: deep-research
    timeout: 120
```

When a context does not specify these values, the global defaults from the `defaults` section apply:

```yaml
defaults:
  timeout: 30
  workstream: default
```

## Example: multi-environment setup

Here is a complete `client.yaml` for a setup with local development, staging, and production servers:

```yaml
api-version: v1
kind: ClientConfig

current-context: dev

contexts:
  - name: dev
    server: http://localhost:8080
    workstream: scratch
    timeout: 10

  - name: staging
    server: https://arawn-staging.internal:8443
    auth:
      type: api-key
      key-file: ~/.config/arawn/keys/staging.key
    workstream: qa
    timeout: 30

  - name: prod
    server: https://arawn.example.com
    auth:
      type: bearer
      token-env: ARAWN_PROD_TOKEN
    workstream: default
    timeout: 60

defaults:
  timeout: 30
  workstream: default
```

Daily workflow:

```sh
# Develop locally
arawn config use-context dev
arawn ask "Summarize today's notes"

# Test against staging
arawn --context staging ask "Run health diagnostics"

# Quick check on production
arawn --context prod status

# Switch to production for a session
arawn config use-context prod
arawn chat
```

## Tips

- **Keep key files out of version control.** Add `~/.config/arawn/keys/` to your global gitignore.
- **Use environment variables in CI.** Set `ARAWN_SERVER_URL` and `ARAWN_PROD_TOKEN` in your CI environment instead of relying on `client.yaml`.
- **One context per trust boundary.** Do not reuse the same API key across staging and production contexts.
- **Verify before switching.** Run `arawn --context <name> status` before `use-context` to confirm the server is reachable.
