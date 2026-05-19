# Recover from LLM warmup failure

*How-to. When `arawn serve` logs `ERROR LLM warmup failed`, this page tells you why and how to fix it.*

On startup arawn probes your configured model (sends a tiny prompt to confirm the provider is reachable and the model exists). If the probe fails, the server log shows the upstream error body. The server keeps running — your next message triggers a fresh attempt — but until you fix the cause, every turn will hit the same error.

## Prerequisites

- arawn server running (or attempting to).
- Access to the terminal where `arawn serve` is logging.

## Match your error body

| Error body contains | Cause | Fix |
|---|---|---|
| `HTTP 401` / `HTTP 403` | API key missing or invalid | Verify `api_key_env` matches the env var you exported and the key value is correct. If using `api_key` in TOML directly, check for typos. |
| `HTTP 403: subscription required` | Model needs a paid plan you don't have | Pick a different model or upgrade. Ollama Cloud has a free tier with smaller models; Groq's free tier covers most public models. |
| `HTTP 404: model … not found` | Model name typo or unavailable on this provider | Check the provider's model list. Common slip: omitting the `openai/` prefix on Groq's gpt-oss models, or using a tag that's been retired. |
| `Connection refused` | Provider URL wrong, or local Ollama not running | Check `provider` in `arawn.toml`. For local Ollama, run `ollama serve` in another terminal. |

## "no API key set for ENVVAR"

You declared `api_key_env = "FOO"` in `arawn.toml` but didn't `export FOO=...` in the shell where `arawn serve` runs.

**Fix.** Either export the variable before starting the server, or switch to `api_key = "..."` in TOML for the value directly. If your provider doesn't need a key (local Ollama), set `api_key_env = ""`.

## "embedding model unavailable — memory system will use FTS only"

This is a warning, not a fatal error. The memory system uses sentence embeddings for semantic search; without the embedder model file, it falls back to keyword (FTS) search. Memory still works, just less semantically clever.

**Fix.** Install the model file at:

```
~/.arawn/models/all-MiniLM-L6-v2/model.onnx
```

The model isn't bundled in the arawn binary to keep size down. Automatic install is on the roadmap.

## TUI shows "connection refused" or hangs

The TUI talks to the server over WebSocket. If the server isn't running, or it's running on a different port than the TUI expects, the TUI looks like it's hung.

**Fix.**

- Confirm `arawn serve` is running in another terminal (check the log).
- If you changed `[server].port` in `arawn.toml`, point the TUI at the new port:

```sh
arawn tui --url ws://127.0.0.1:<your-port>/ws
```

## What's next

- If you suspect the model name is wrong, the provider's docs are the source of truth. Common providers' model lists:
  - Groq: <https://console.groq.com/docs/models>
  - Ollama: <https://ollama.com/library>
  - OpenAI: <https://platform.openai.com/docs/models>
  - Anthropic: <https://docs.anthropic.com/claude/docs/models-overview>
- Full configuration schema: [config schema reference](../reference/config-schema.md).
- Switching providers without restarting: [`/llm` slash command](../reference/slash-commands.md) (on backlog).
