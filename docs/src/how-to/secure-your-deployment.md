# Secure Your Deployment

This guide covers the security features available in Arawn and how to configure them for different deployment scenarios. Arawn is designed to run as a personal agent, but exposing it on a network requires careful configuration to prevent unauthorized access and abuse.

## Authentication

### Token-based authentication

The simplest way to protect your server is with a bearer token. Set it at startup:

```sh
arawn start --token my-secret-token
```

Or use an environment variable (useful for service files and containers):

```sh
export ARAWN_API_TOKEN=my-secret-token
arawn start
```

When a token is configured, all API requests must include it in the `Authorization` header:

```sh
curl -H "Authorization: Bearer my-secret-token" http://localhost:8080/health
```

Token comparison uses constant-time comparison via the `subtle` crate to prevent timing attacks. The comparison takes the same amount of time regardless of how many characters match, preventing an attacker from guessing the token character by character.

### Generate a token

Use the built-in token generator to create a cryptographically random token:

```sh
arawn auth token --generate
```

Copy the output and use it as your `--token` value or `ARAWN_API_TOKEN`.

### Localhost-only mode

When no token is configured, Arawn operates in localhost-only mode. It binds to `127.0.0.1` by default and accepts all requests without authentication. If you bind to a non-loopback address (e.g., `0.0.0.0`) without setting a token, the server logs a warning because the API would be accessible to anyone on the network.

## CORS configuration

If you access the Arawn API from a web browser (for example, through a custom UI), configure allowed origins in `config.toml`:

```toml
[server]
cors_origins = [
    "https://my-ui.example.com",
    "http://localhost:3000",
]
```

When the list is empty (the default), no CORS headers are sent.

## Rate limiting

Rate limiting prevents abuse by capping the number of requests per IP address per minute. It is enabled by default.

### Configuration

Set rate limits in `config.toml`:

```toml
[server]
rate_limiting = true
api_rpm = 120     # General API endpoints: 120 requests/minute/IP
```

The chat endpoint uses a separate, lower limit of 60 requests per minute per IP (`chat_rpm`).

Rate limiting uses the `governor` crate with per-IP keyed rate limiters. When a client exceeds the limit, the server returns `429 Too Many Requests` with a `Retry-After` header.

### Disable rate limiting

For development or trusted environments:

```toml
[server]
rate_limiting = false
```

## WebSocket security

### Origin restrictions

Control which origins can open WebSocket connections:

```toml
[server]
ws_allowed_origins = [
    "https://my-app.example.com",
    "*.example.com",
]
```

Wildcard subdomain matching is supported. When the list is empty and authentication is enabled, only localhost origins are accepted. Set explicit origins for any browser-based client.

### Connection rate limiting

WebSocket connections are rate-limited separately at 30 connections per minute per IP address. This prevents connection flood attacks where an attacker rapidly opens and closes connections.

### Message size limits

| Limit | Default | Purpose |
|-------|---------|---------|
| WebSocket message | 1 MB | Prevents memory exhaustion from oversized messages |
| REST request body | 10 MB | Caps file uploads and large payloads |

These defaults are defined in the server configuration and can be adjusted via the `ServerConfig` builder in custom deployments.

## Tailscale integration

For zero-config secure access across your devices, Arawn integrates with Tailscale identity headers. When a request arrives from the Tailscale CGNAT IP range (`100.64.0.0/10`) or loopback, the server trusts the `Tailscale-User-Login` header set by the Tailscale proxy.

### Configure allowed Tailscale users

In `config.toml`:

```toml
[server]
tailscale_users = [
    "alice@example.com",
    "bob@example.com",
]
```

Requests with a `Tailscale-User-Login` header matching an allowed user are authenticated without requiring a bearer token. The server verifies that the source IP is within the CGNAT range or is loopback before trusting the header, preventing header spoofing from non-Tailscale sources.

Bearer token authentication takes precedence over Tailscale authentication when both headers are present.

## Proxy trust

By default, Arawn extracts client IP addresses from the TCP connection (`ConnectInfo`), ignoring proxy headers entirely. This is the safe default.

If you run Arawn behind a reverse proxy (nginx, Caddy, Traefik), enable proxy trust so rate limiting and logging use the real client IP:

```toml
[server]
trust_proxy = true
```

When enabled, the server reads client IP from these headers in order:

1. `X-Forwarded-For` (first IP if multiple are chained)
2. `X-Real-IP`
3. TCP connection address (fallback)

**Only enable `trust_proxy` behind an authenticated reverse proxy.** If exposed directly, an attacker can spoof `X-Forwarded-For` to bypass per-IP rate limiting.

## Secret storage

Arawn uses age encryption to store secrets at rest. Secrets (API keys, tokens) are stored as an encrypted JSON map in `~/.config/arawn/secrets.age`, encrypted with a per-user age identity key at `~/.config/arawn/identity.age`.

### Store a secret

```sh
arawn secrets set anthropic
```

You will be prompted for the API key, which is encrypted and stored. The decrypted value is never logged or written to disk in plaintext.

### How secrets are resolved

When the agent needs an API key, it resolves secrets through the `SecretResolver` interface. The encrypted store is decrypted in memory, the secret is read, and the cleartext is passed directly to the API client. Resolved secret values are never included in log output.

## SSRF protection

The web tools (HTTP fetch, web search) include Server-Side Request Forgery (SSRF) protection. Before making any outbound HTTP request, Arawn resolves the target hostname and checks the resulting IP addresses against restricted ranges:

| Range | Description |
|-------|-------------|
| `127.0.0.0/8` | Loopback |
| `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16` | Private networks |
| `169.254.0.0/16` | Link-local (includes cloud metadata endpoints) |
| `100.64.0.0/10` | CGNAT / Tailscale |
| `::1`, `fc00::/7` | IPv6 loopback and unique local |

This prevents the agent from being tricked into accessing internal services, cloud instance metadata endpoints (`169.254.169.254`), or other resources that should not be reachable from outbound HTTP requests.

## Command validator

Before executing any shell command, Arawn runs it through a command validator that blocks dangerous patterns. This is a defense-in-depth layer that operates before the OS-level sandbox.

Blocked patterns include:

| Pattern | Example |
|---------|---------|
| Root filesystem deletion | `rm -rf /`, `rm -rf /*` |
| Fork bombs | `:(){ :|:& };:` |
| Raw device access | `dd if=/dev/sda`, `> /dev/sda` |
| Filesystem creation | `mkfs` |
| System control | `shutdown`, `reboot`, `halt`, `poweroff` |
| Sandbox escape attempts | `sandbox-exec`, `bwrap`, `unshare`, `nsenter`, `chroot` |
| Kernel module manipulation | `insmod`, `rmmod`, `modprobe` |

The validator uses regex matching to be precise. For example, `rm -rf /tmp/build` is allowed because it targets a subdirectory, but `rm -rf /` is blocked.

## OS-level sandbox

The command validator is the inner defense layer. The primary security boundary is the OS-level sandbox, which restricts what the agent process can do at the kernel level.

### macOS: sandbox-exec (Seatbelt)

On macOS, Arawn uses `sandbox-exec` with a Seatbelt profile that restricts filesystem access, network access, and process execution. Commands run by the agent are wrapped in a sandbox that limits them to the working directory and temporary files.

### Linux: bubblewrap

On Linux, Arawn uses bubblewrap (`bwrap`) to create a sandboxed environment with restricted filesystem mounts, no network access (unless explicitly allowed), and no privilege escalation.

The platform is detected automatically at runtime. On unsupported platforms, the sandbox layer is skipped and only the command validator provides protection.

## Deployment checklist

Use this checklist when exposing Arawn beyond localhost:

- [ ] Set a strong authentication token (`--token` or `ARAWN_API_TOKEN`)
- [ ] Generate the token cryptographically (`arawn auth token --generate`)
- [ ] Verify rate limiting is enabled (`rate_limiting = true`)
- [ ] Configure CORS origins if using a web UI
- [ ] Set WebSocket allowed origins for browser clients
- [ ] Keep `trust_proxy = false` unless behind a reverse proxy
- [ ] Store API keys in the encrypted secret store, not in environment variables
- [ ] Back up `identity.age` -- losing it means losing access to all encrypted secrets
- [ ] Review the command validator is active (it is enabled by default)
- [ ] If using Tailscale, configure `tailscale_users` to restrict access

## Example: secure remote access with Tailscale

This configuration allows access from your Tailscale network without managing tokens, while still rate-limiting and restricting WebSocket origins:

```toml
[server]
bind = "0.0.0.0"
port = 8080
rate_limiting = true
api_rpm = 120

tailscale_users = [
    "you@example.com",
]

ws_allowed_origins = [
    "https://arawn.your-tailnet.ts.net",
]
```

```sh
arawn start
```

Access from any device on your tailnet:

```sh
arawn --server http://arawn.your-tailnet.ts.net:8080 status
```
