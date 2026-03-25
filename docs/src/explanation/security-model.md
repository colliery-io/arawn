# Security Model

Arawn gives an LLM the ability to execute shell commands, read and write files, and make network requests on your machine. That makes security non-negotiable. This page explains the defense-in-depth approach: eight layers that work together so that no single failure compromises the system.

## Why Defense-in-Depth?

A single security boundary is a single point of failure. If the sandbox has a bug, everything inside is exposed. If authentication is bypassed, every endpoint is accessible. Defense-in-depth means that even when one layer fails, the others contain the damage.

Each layer in Arawn's security model addresses a different threat:

| Layer | Threat | Mechanism |
|-------|--------|-----------|
| Authentication | Unauthorized access to the API | Bearer tokens, Tailscale identity |
| Rate Limiting | Denial of service, resource exhaustion | Per-IP token bucket (governor) |
| Input Validation | Oversized payloads, malformed input | Size limits on messages, bodies, WebSocket frames |
| FsGate | Agent accessing files outside its workspace | Path validation, workstream boundary enforcement |
| CommandValidator | Destructive shell commands | Regex blocklist with normalization |
| OS Sandbox | Arbitrary code execution escaping containment | macOS sandbox-exec, Linux bubblewrap |
| SSRF Protection | Agent making requests to internal services | DNS resolution check against private ranges |
| Secret Management | Credential exposure in logs or tool output | Age encryption at rest, handle-based resolution |

## Layer 1: Authentication

When a client connects to the Arawn server, the first question is: should this request be allowed at all?

**Bearer token authentication** is the primary mechanism. The server is configured with an auth token, and every request must include it in the `Authorization: Bearer <token>` header. Token comparison uses the `subtle` crate's constant-time equality check to prevent timing attacks. An attacker who can measure response times cannot incrementally guess the token byte by byte.

**Tailscale identity** is the secondary mechanism. When configured with a list of allowed Tailscale users, the server accepts the `Tailscale-User-Login` header -- but only from IP addresses in the Tailscale CGNAT range (100.64.0.0/10) or loopback. This source IP check prevents an attacker on the open internet from spoofing the Tailscale header. Without it, anyone who knew the header name could impersonate a Tailscale user.

**Localhost mode** applies when no auth token is configured. In this mode, the server accepts all requests. This is appropriate for local development where the server is bound to 127.0.0.1 and only accessible from the same machine.

The authentication middleware is applied globally. Bearer token takes precedence when both headers are present, and the authenticated identity is injected into request extensions for downstream handlers to inspect.

## Layer 2: Rate Limiting

Even authenticated users can cause problems if they send too many requests. Rate limiting prevents both accidental flooding (a script in a tight loop) and intentional abuse.

Arawn uses the `governor` crate to implement per-IP token bucket rate limiting. Each IP address gets an independent quota:

- **Chat endpoints**: 60 requests per minute
- **General API endpoints**: 120 requests per minute
- **WebSocket connections**: 30 per minute

The per-IP design is important. A misbehaving client affects only itself -- other users continue to be served normally. The rate limiter is keyed by the actual client IP, extracted from `ConnectInfo` by default. When running behind a reverse proxy, `trust_proxy` can be enabled to read `X-Forwarded-For`, but this is disabled by default to prevent IP spoofing attacks.

## Layer 3: Input Validation

Before any request reaches business logic, its size is checked:

- **Maximum message size**: 100 KB
- **Maximum request body**: 10 MB
- **Maximum WebSocket message**: 1 MB

These limits exist to prevent memory exhaustion. A malicious or buggy client sending a 1 GB message body could crash the server. By rejecting oversized payloads at the transport layer, the rest of the system never sees them.

## Layer 4: The FsGate Trait

The `FsGate` trait is the central abstraction for filesystem access control. It defines four methods:

```rust
pub trait FsGate: Send + Sync {
    fn validate_read(&self, path: &Path) -> Result<PathBuf, FsGateError>;
    fn validate_write(&self, path: &Path) -> Result<PathBuf, FsGateError>;
    fn working_dir(&self) -> &Path;
    async fn sandbox_execute(&self, command: &str, timeout: Option<Duration>)
        -> Result<SandboxOutput, FsGateError>;
}
```

The gate is shared as `Arc<dyn FsGate>` and injected into the tool execution context. Every gated tool (file_read, file_write, glob, grep, shell) must pass through it. If no gate is configured, gated tools are denied by default with a clear error message explaining that they require a workstream context.

### WorkstreamFsGate

The primary implementation of `FsGate` is `WorkstreamFsGate`, which scopes filesystem access to workstream boundaries:

- **Named workstreams**: Tools can access `production/` (for outputs) and `work/` (for scratch space). All sessions within the same workstream share these directories.
- **Scratch workstreams**: Tools are isolated to `scratch/sessions/<session-id>/work/`. Each session gets its own directory. Session A cannot read session B's files.

### PathValidator

Inside `WorkstreamFsGate`, the `PathValidator` performs three checks:

1. **Allowed list**: The path must be a descendant of one of the allowed directories.
2. **Denied paths**: Sensitive system paths (like `/etc/shadow`) are explicitly blocked regardless of the allowed list.
3. **Symlink detection**: The validator canonicalizes paths and checks whether symlinks escape the sandbox boundary. A symlink at `/workstream/work/escape` pointing to `/etc/passwd` will be caught.

### Why a Trait?

Making `FsGate` a trait rather than a concrete type serves two purposes. First, it allows different workstream types to implement different policies without the tool execution pipeline knowing or caring about the differences. Second, it makes the system testable -- tests use `MockFsGate` to simulate various access patterns without touching the real filesystem.

## Layer 5: CommandValidator

Before a shell command reaches the OS-level sandbox, it passes through the `CommandValidator`. This is a regex-based blocklist that catches clearly dangerous commands early and returns a human-readable error.

The validator blocks:
- **Root filesystem destruction**: `rm -rf /` and `rm -rf /*` (but allows `rm -rf /tmp/build`)
- **Fork bombs**: `:(){ :|:& };:`
- **Raw device access**: `dd if=/dev/...`, `> /dev/sda`
- **Filesystem destruction**: `mkfs` commands
- **System control**: `shutdown`, `reboot`, `halt`, `poweroff`, `init 0/6`
- **Sandbox escape attempts**: `sandbox-exec`, `bwrap`, `unshare`, `nsenter`, `chroot`, `csrutil`
- **Kernel module manipulation**: `insmod`, `rmmod`, `modprobe`
- **Process tracing**: `ptrace`, `strace -p`, `gdb --pid`, `lldb --attach`

### Normalization

LLMs are creative about how they construct commands. The validator applies five normalization passes before matching:

1. **Lowercase**: `RM -RF /` becomes `rm -rf /`
2. **Quote removal**: `"rm" "-rf" "/"` becomes `rm -rf /`
3. **Backslash escape removal**: `r\m -rf /` becomes `rm -rf /`
4. **Whitespace collapse**: `rm   -rf   /` becomes `rm -rf /`
5. **Basename extraction**: `/usr/bin/rm -rf /` becomes `rm -rf /`

This is explicitly defense-in-depth. The CommandValidator is not the security boundary -- the OS sandbox is. The validator catches obvious dangers with clear error messages so the agent can try a different approach, rather than having the sandbox silently fail.

## Layer 6: OS Sandbox

The `SandboxManager` provides OS-level process isolation for shell command execution. This is the primary security boundary for tool execution.

**On macOS**, Arawn uses `sandbox-exec` with a Scheme-based profile that:
- Denies all file writes by default
- Allows writes only to explicitly listed paths (the workstream directories)
- Uses proxy-based network domain filtering

**On Linux**, Arawn uses `bubblewrap` (bwrap) with `socat` for:
- Mount namespace isolation
- Read-only bind mounts for system directories
- Writable bind mounts only for allowed paths
- Network filtering via proxy

The sandbox is not optional. If the sandbox runtime cannot be initialized (missing dependencies), shell execution returns an error rather than falling back to unsandboxed execution. The error message includes installation instructions for the missing components.

Write access inside the sandbox is restricted to the paths configured by the `FsGate`. Even if the LLM constructs a command that writes to an unexpected location, the sandbox's filesystem policy will block it at the kernel level.

## Layer 7: SSRF Protection

The agent can make HTTP requests (via web_search and web_fetch tools). Without protection, it could be tricked into requesting internal services: `http://169.254.169.254/latest/meta-data/` (cloud metadata), `http://localhost:8080/admin` (local services), or any other address on the private network.

Before any outbound HTTP request, Arawn resolves the target URL's hostname to an IP address and checks it against blocked ranges:

- **Loopback**: 127.0.0.0/8
- **Private**: 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
- **Link-local**: 169.254.0.0/16
- **CGNAT**: 100.64.0.0/10

If the resolved IP falls in any of these ranges, the request is rejected before a TCP connection is established.

## Layer 8: Secret Management

Secrets (API keys, tokens, credentials) need special handling because they flow through many system components: configuration, tool parameters, shell environments, and log output.

**At rest**, secrets are encrypted using `age` (a modern file encryption tool). The encrypted file lives at `~/.config/arawn/secrets.age`. The age identity (private key) is stored separately at `~/.config/arawn/identity.age`. Both files are created with 0o600 permissions (owner read/write only). Writes use atomic rename (write to `.tmp`, then rename) to prevent corruption from crashes.

**In transit**, secrets are referenced by handle rather than by value. Tool parameters use the `${{secrets.name}}` syntax. The actual value is resolved at the last possible moment -- inside `ToolRegistry.execute_with_config()`, just before the tool runs. The original parameters (with handles, not values) are what get logged and stored in conversation history. This means secret values never appear in logs, session JSONL files, or memory stores.

**The `SecretResolver` trait** abstracts secret resolution so the tool execution pipeline does not depend on any particular storage backend. The `AgeSecretStore` implements it, but tests use a `MockSecretResolver`. This separation means the security-critical secret resolution logic can be tested independently.

## Default-Deny Philosophy

The security model follows a default-deny philosophy throughout:

- No auth token configured? Localhost-only access (still secure by network topology).
- No FsGate on the context? Gated tools are denied with a clear error.
- Sandbox unavailable? Shell execution fails rather than running unprotected.
- URL resolves to private IP? Request blocked before connecting.
- Unknown `${{secrets.name}}`? Handle left as-is (does not leak, does not crash).

Every security decision defaults to the safe option. Features must be explicitly enabled rather than implicitly available.

## How the Layers Compose

Consider what happens when the agent tries to execute `cat /etc/shadow`:

1. **Authentication**: The request was already authenticated at the HTTP layer.
2. **Rate limiting**: The request was within rate limits.
3. **FsGate**: The `shell` tool is gated. The gate is present (we are in a workstream).
4. **CommandValidator**: `cat /etc/shadow` passes validation (it is not in the blocklist -- reading a file is not inherently destructive).
5. **OS Sandbox**: The command runs inside the sandbox. The sandbox's filesystem policy blocks read access to `/etc/shadow`. The command fails with "Operation not permitted."
6. **Output**: The error message is returned to the agent, which can then try a different approach.

No single layer blocked this attack alone. The CommandValidator did not catch it because reading files is a legitimate operation. But the OS sandbox did, because `/etc/shadow` is not in the allowed read paths. This is defense-in-depth in action.
