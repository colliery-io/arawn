# Troubleshooting

*Reference. Symptom → cause → fix tables for the common failure modes.*

For deeper diagnosis on specific subsystems, see the related how-to: [debug OAuth failures](../how-to/debug-oauth-failures.md), [recover from LLM warmup failure](../how-to/recover-from-llm-warmup-failure.md).

## Server startup

| Symptom | Cause | Fix |
|---|---|---|
| `ERROR LLM warmup failed` + `HTTP 401` / `HTTP 403` | API key missing or invalid | Verify `api_key_env` matches the exported var; check key value. |
| `ERROR LLM warmup failed` + `HTTP 403: subscription required` | Model needs a paid plan | Switch model or upgrade. |
| `ERROR LLM warmup failed` + `HTTP 404: model … not found` | Model name typo or unavailable | Check provider's model list; common slip: missing `openai/` prefix for Groq gpt-oss models. |
| `ERROR LLM warmup failed` + `Connection refused` | Provider URL wrong or local Ollama not running | Check `[llm.<name>].provider`; for local Ollama, `ollama serve`. |
| `no API key set for ENVVAR` | `api_key_env = "FOO"` set but `FOO` not exported | Export it, or use `api_key = "..."` direct value, or `api_key_env = ""` for keyless. |
| `embedding model unavailable — memory system will use FTS only` | Embedder model file missing | Install at `<data_dir>/models/all-MiniLM-L6-v2/model.onnx`. (Non-fatal — memory falls back to FTS.) |
| `Address already in use` on `arawn serve` start | Another process on `[server].port` (default 3100) | Stop it or change port. |
| `bubblewrap: not found` (Linux) | `bwrap` binary missing | `apt install bubblewrap` / `pacman -S bubblewrap`. Shell tool fails closed until installed. |

## TUI

| Symptom | Cause | Fix |
|---|---|---|
| TUI shows "connection refused" or hangs | `arawn serve` not running, or wrong port | Start the server. Check `[server].port`; pass `arawn tui --url ws://127.0.0.1:<port>/ws` if non-default. |
| TUI shows but `/help` doesn't list `/connect` | Server is older than TUI (or vice versa) | Rebuild both: `cargo build --release`. |
| `/connect <svc>` opens browser then "Connection refused" in browser | Port 8080 occupied (Slack/Atlassian require it) | Stop conflicting process: `lsof -iTCP:8080 -sTCP:LISTEN`. |

## OAuth

See [debug OAuth failures](../how-to/debug-oauth-failures.md) for the full table.

| Symptom | Cause | Fix |
|---|---|---|
| `Error 400: redirect_uri_mismatch` | Allowed redirect URI doesn't match | Slack/Atlassian: `http://localhost:8080/oauth/callback` exactly (use `localhost` not `127.0.0.1`). Google Desktop apps: any localhost. |
| `Error 403: access_denied` | Clicked Deny or not on test-users list | Re-try; add yourself as test user. |
| `insufficient_scope` | Added scope after first `/connect` | `/disconnect <svc>`, `/connect <svc>`. For Slack also re-install the app. |
| `invalid_grant` | Refresh token expired or revoked | `/disconnect <svc>`, `/connect <svc>`. |

## Feeds and palaces

| Symptom | Cause | Fix |
|---|---|---|
| `/feeds` shows a feed but `last_status = backfill-rate-limited` | Backfill hit the 5-minute rate-limit cap | Cron will resume from the persisted cursor on the next tick. Be patient or `/feeds run <id>` later. |
| `last_status = auth failed: ...` | Provider token revoked or scope removed | `/disconnect <svc>` then `/connect <svc>`. |
| `last_status = backfill-failed: <reason>` | Backfill couldn't recover from a transient error | Inspect `meta.json` in the feed directory; usually the reason is informative. `/feeds run <id>` to retry. |
| `signal_search` returns nothing on a freshly-bound workstream | Extraction hasn't run yet | `/feeds run <feed_id>` to force an immediate run, then re-query. |
| Disk growing fast under `data/<provider>/...` | Noisy day in the feed (bot run, backup) | Inspect per-day partitions to find the culprit; consider tighter feed parameters. |

## Workflows

| Symptom | Cause | Fix |
|---|---|---|
| `workflow_create` takes ~30 seconds the first time | Rust compiler cache cold | Subsequent workflow_creates are faster. |
| Workflow doesn't fire on schedule | Wrong cron / timezone | Check `workflow_status <name>`; cron is standard 5-field UTC unless `cron_timezone` set. |
| Workflow runs but errors with `failed to load <name>.dylib` | Plugin install dir wrong, or stale on-disk dylib | `workflow_delete <name>` then re-create. |

## Permission denials

| Symptom | Cause | Fix |
|---|---|---|
| Agent gets denied on every shell call | `permission_mode = "plan"` is active | `/accept on` (or `/accept off` for default mode). |
| Tool keeps prompting even though you said "always allow" | Session grants don't persist across server restarts | Add an explicit `allow` rule in `[permissions]` to make it permanent. |
| `shell(...)` returns "tool denied (by rule)" | A `deny` rule matched | Inspect with `/permissions` slash. |

## When debugging isn't yielding

Run the server with verbose logging:

```sh
RUST_LOG=arawn=debug,arawn_integrations=trace arawn serve
```

Logs land at `<data_dir>/logs/server.log` (daily rotation). The full env-filter syntax is in [tracing-subscriber's docs](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html).

For first-principle diagnosis:

```sh
arawn doctor --json
```

Walks through config validation, integration credentials, sandbox availability, embedder model presence, data directory layout, and provider reachability.
