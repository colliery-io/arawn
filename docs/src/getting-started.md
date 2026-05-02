# Getting Started

This walkthrough takes you from `git clone` to a working chat session.
Estimated time: ten minutes if you already have a Rust toolchain and an LLM
provider account.

## Prerequisites

- **Rust toolchain** — `rustup` with a stable Rust ≥ 1.80 (`rustup default stable`).
- **An LLM provider** — pick one:
  - **Groq** (free tier, fast) — sign up at [console.groq.com](https://console.groq.com),
    create an API key.
  - **Ollama Cloud** (paid for some models) — sign up at
    [ollama.com](https://ollama.com), create an API key. Lets you reach large
    open models without local GPU.
  - **Local Ollama** — install [ollama](https://ollama.com), `ollama pull
    llama3.1:8b`. No API key needed.
  - **OpenAI**, **Anthropic**, or anything OpenAI-compatible (vLLM,
    LM Studio, Together, Fireworks, Mistral, ...) — works the same way.

## 1. Build

```sh
git clone <repo-url> arawn
cd arawn
cargo build --release
```

The binary lands at `target/release/arawn`. Either add it to your `PATH` or
invoke it by full path in the steps below.

## 2. Configure

Arawn reads `~/.arawn/arawn.toml`. Pick the example matching your provider:

### Option A: Groq

```toml
# ~/.arawn/arawn.toml

[llm.default]
provider = "groq"
model = "openai/gpt-oss-120b"
api_key_env = "GROQ_API_KEY"

[engine]
llm = "default"

[server]
host = "127.0.0.1"
port = 3100

[storage]
data_dir = "~/.arawn"
```

```sh
export GROQ_API_KEY=gsk_your_key_here
```

### Option B: Ollama Cloud

```toml
# ~/.arawn/arawn.toml

[llm.default]
provider = "https://ollama.com/v1"
model = "gemma4:31b-cloud"
api_key_env = "OLLAMA_API_KEY"

[engine]
llm = "default"

[server]
host = "127.0.0.1"
port = 3100

[storage]
data_dir = "~/.arawn"
```

```sh
export OLLAMA_API_KEY=your_key_here
```

> Available cloud models depend on your account. `gemma4:31b-cloud`,
> `qwen3-coder:480b-cloud`, `gpt-oss:120b-cloud`, and `deepseek-v3.1:671b-cloud`
> are commonly available; some models (e.g. `deepseek-v4-pro:cloud`) require
> a subscription. If a model is unavailable, arawn's startup warmup will tell
> you immediately — see [Troubleshooting](#troubleshooting) below.

### Option C: Local Ollama

```toml
[llm.default]
provider = "ollama"
model = "llama3.1:8b"
api_key_env = ""    # local Ollama needs no auth

[engine]
llm = "default"
```

No env var needed. Make sure `ollama serve` is running.

## 3. Start the server

```sh
arawn serve
```

Watch for these lines:

```
INFO arawn: LLM client pool ready ... engine_model=...
INFO arawn: LLM warmup OK name=default provider=... model=...
```

If you see `ERROR LLM warmup failed`, jump to [Troubleshooting](#troubleshooting).

Leave this terminal open. The server holds your session state.

## 4. Open the TUI

In a second terminal:

```sh
arawn tui
```

You'll see an input area at the bottom of the screen. Type a message and
press `Enter`.

## 5. First message

Try something concrete:

```
What's in this directory?
```

The agent will use its `shell` tool to run `ls`, then summarize. You should
see streaming text appear, then the tool call indicator, then the result.

You're set. From here:

- Read about [the available tools](./intro.md#whats-included) (work in progress).
- Try a more involved prompt: `Find any TODO comments in this repo and group them by file`.
- Press `Ctrl+C` in the TUI to quit; the server keeps running.

## CLI one-shot mode

Once the server is running, you can also send single prompts without the TUI:

```sh
arawn "draft a one-line commit message for the diff in this repo"
```

This streams a single response and exits. Useful for shell scripts.

## Troubleshooting

### "ERROR LLM warmup failed" at server startup

Arawn probes your configured model on startup. If it fails, the server log
shows the upstream error body. Common cases:

| Error body contains | Cause | Fix |
|---|---|---|
| `HTTP 401` / `HTTP 403` | API key missing or invalid | Verify `api_key_env` matches the var you exported, and that the key itself is correct. |
| `HTTP 403: subscription required` | Model needs a paid plan you don't have | Pick a different model or upgrade. |
| `HTTP 404: model … not found` | Model name typo or unavailable on this provider | Check the provider's model list. |
| `Connection refused` | Provider URL wrong, or local Ollama not running | Check `provider` URL; `ollama serve` if local. |

The server keeps running after a failed warmup — your *next* message will
trigger a fresh warmup attempt and the engine will surface the same error
to the TUI if it still fails.

### "embedding model unavailable — memory system will use FTS only"

The memory system uses sentence embeddings for semantic search. If the
embedder model isn't available locally, arawn falls back to keyword/FTS
search — it still works, just less smart.

To fix: install the model file at the path arawn expects (default
`~/.arawn/models/all-MiniLM-L6-v2/model.onnx`). The model isn't bundled to
keep the binary small. (We'll automate this in a future release.)

### TUI shows "connection refused" or hangs

The TUI talks to the server over WebSocket. Make sure `arawn serve` is
running in another terminal and listening on the same port (default 3100).
If you changed `[server].port` in `arawn.toml`, point the TUI at it:

```sh
arawn tui --url ws://127.0.0.1:<your-port>/ws
```

### "no API key set for ENVVAR"

You declared `api_key_env = "FOO"` in `arawn.toml` but didn't `export FOO=…`
in the shell where you ran `arawn serve`. Either export it, or set
`api_key_env = ""` if your provider doesn't need a key (local Ollama).

## Next steps

- Resume an old session: `arawn --list-sessions`, then `arawn --session <uuid>`.
- See what tools the agent has: in the TUI, type `/help` (some commands
  are work-in-progress — see backlog T-0195).
- Build a scheduled workflow: see the workflow docs (work in progress —
  see backlog T-0198).
