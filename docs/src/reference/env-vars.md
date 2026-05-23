# Environment variables

*Reference. Every environment variable arawn reads.*

Source: `grep env::var` across `crates/`.

## Runtime

| Variable | Read by | Effect |
|---|---|---|
| `ARAWN_DATA_DIR` | `crates/arawn/src/main.rs` | Data directory root. Overrides `[storage].data_dir`. Equivalent to `--data-dir <path>`. |
| `ARAWN_TOOL_TIMEOUT_SECS` | `crates/arawn-engine/src/tool_timeout.rs` | Default wall-clock timeout per tool call (seconds). Overrides `[engine].tool_timeout_secs`. |
| `RUST_LOG` | `tracing_subscriber` | Standard `tracing` env filter. Useful patterns: `arawn=debug` (all arawn crates) or `arawn_integrations=trace` (per-crate). |
| `SHELL` | `crates/arawn/src/main.rs` | Used to enrich the system prompt's environment context. Falls back to `/bin/sh`. |
| `TMPDIR` | `crates/arawn-engine/src/tools/shell.rs` | Shell sandbox temp directory. Standard POSIX env var. |

## LLM provider API keys

The `[llm.<name>].api_key_env` config key tells arawn which variable to read. Default profile expects `GROQ_API_KEY`. Other commonly-used names:

| Variable | Provider |
|---|---|
| `GROQ_API_KEY` | Groq (default). |
| `OPENAI_API_KEY` | OpenAI (read by `arawn-llm/src/openai_compat.rs`). |
| `ANTHROPIC_API_KEY` | Anthropic. |
| `OLLAMA_API_KEY` | Ollama Cloud. |

`GROQ_MODEL` is read once by the default-config generator as a convenience seed. Otherwise model selection is via `[llm.<name>].model`.

## Integration OAuth credentials

These override `[integrations.<service>].{client_id,client_secret}` at startup. Use them when you don't want plaintext OAuth secrets in `arawn.toml`.

| Variable | Service |
|---|---|
| `ARAWN_GMAIL_CLIENT_ID`, `ARAWN_GMAIL_CLIENT_SECRET` | Gmail-specific OAuth client (falls back to `ARAWN_GOOGLE_*`). |
| `ARAWN_GOOGLE_CLIENT_ID`, `ARAWN_GOOGLE_CLIENT_SECRET` | Shared Google OAuth client (Gmail/Calendar/Drive). |
| `ARAWN_GCAL_CLIENT_ID`, `ARAWN_GCAL_CLIENT_SECRET` | Google Calendar-specific (falls back to `ARAWN_GOOGLE_*`). |
| `ARAWN_GDRIVE_CLIENT_ID`, `ARAWN_GDRIVE_CLIENT_SECRET` | Google Drive-specific (falls back to `ARAWN_GOOGLE_*`). |
| `ARAWN_SLACK_CLIENT_ID`, `ARAWN_SLACK_CLIENT_SECRET` | Slack OAuth client. |
| `ARAWN_ATLASSIAN_CLIENT_ID`, `ARAWN_ATLASSIAN_CLIENT_SECRET` | Atlassian 3LO client. |

## GitHub App credentials

GitHub uses an App, not OAuth. Three env vars (path form preferred):

| Variable | Description |
|---|---|
| `ARAWN_GITHUB_APP_ID` | Numeric App ID. |
| `ARAWN_GITHUB_APP_SLUG` | URL slug. |
| `ARAWN_GITHUB_PRIVATE_KEY_PATH` | Path to the App's RSA private key in PEM format. |
| `ARAWN_GITHUB_PRIVATE_KEY_PEM` | Inline PEM body (alternative to `_PATH`; less common). |

## Resolution order

For OAuth credentials: env var > `arawn.toml` > integration disabled. If both env and TOML are set, env wins.

For LLM keys: `api_key` (TOML, direct value) > `api_key_env` (TOML, name of env var to read) > not set.
