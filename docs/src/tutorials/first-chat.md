# Your first chat session

*Tutorial — about ten minutes from `git clone` to a working agent conversation. No prior arawn knowledge required.*

By the end of this tutorial you'll have arawn running locally, the TUI open, and the agent answering questions about your filesystem. You won't connect any external services yet — that's the next set of how-to guides. The goal here is to feel how the agent loop works.

## What you need

- **Rust toolchain.** `rustup` with stable Rust ≥ 1.80 (`rustup default stable`).
- **An LLM provider account.** This tutorial uses **Groq** — it has a free tier and is fast. Sign up at [console.groq.com](https://console.groq.com) and create an API key. (You can use OpenAI, Ollama Cloud, local Ollama, Anthropic, or any OpenAI-compatible provider instead — see [configuration schema](../reference/config-schema.md) for the full set.)

That's it. No OAuth, no integrations, no databases to set up — arawn ships as a single binary with embedded SQLite.

## 1. Build

```sh
git clone <repo-url> arawn
cd arawn
cargo build --release
```

The binary lands at `target/release/arawn`. Either add it to your `PATH` or run it by full path in the steps below.

## 2. Configure

arawn reads `~/.arawn/arawn.toml`. Create it with this minimal Groq configuration:

```toml
# ~/.arawn/arawn.toml

[llm.default]
provider = "groq"
model = "openai/gpt-oss-120b"
api_key_env = "GROQ_API_KEY"

[engine]
llm = "default"
```

Then export your API key:

```sh
export GROQ_API_KEY=gsk_your_key_here
```

If you'd rather keep the key out of your shell environment, replace `api_key_env` with `api_key = "gsk_..."` in the TOML and skip the export.

## 3. Start the server

```sh
arawn serve
```

You should see something like:

```
INFO arawn: LLM client pool ready ... engine_model=...
INFO arawn: LLM warmup OK name=default provider=groq model=openai/gpt-oss-120b
```

The warmup line confirms arawn can reach Groq and the model exists. If you instead see `ERROR LLM warmup failed`, see [recover from LLM warmup failure](../how-to/recover-from-llm-warmup-failure.md).

Leave this terminal open. The server holds your session state.

## 4. Open the TUI

In a second terminal:

```sh
arawn tui
```

You'll see an input area at the bottom. Type a message and press `Enter`.

## 5. Your first message

Try a question the agent can answer by looking around your filesystem:

```
What's in this directory?
```

The agent will reach for its `shell` tool, ask your permission to run `ls` (the first time), then summarize what it finds.

Three things just happened:

1. **The LLM saw your message** plus a system prompt explaining what tools it has available.
2. **It chose a tool** — `shell` — and a command, then waited for permission.
3. **You granted permission** and the result fed back into the LLM, which composed the human-readable answer you see.

That tool-call → permission → execute → respond loop is the whole agent. Everything else arawn does — integrations, lenses, ceremonies — is variations on it.

## What's next

Now that you have a working session:

- **Connect external tools.** The agent gets a lot more useful when it can read your Gmail, Slack, Calendar, or GitHub. Pick one and follow the matching how-to: [Google](../how-to/connect-google.md), [Slack](../how-to/connect-slack.md), [Atlassian](../how-to/connect-atlassian.md), [GitHub](../how-to/connect-github.md).
- **Track a lens end-to-end.** Once you have at least one integration connected, work through [your first lens](./first-lens.md) — it walks through create → bind → extraction → `signal_search` and shows the three-layer data model in action.
- **Look something up.** The [reference](../reference/index.md) catalogs every CLI flag, slash command, config key, and agent tool.
- **Understand why arawn works this way.** The [explanation](../explanation/index.md) pages cover the agent loop, the data model, and the design decisions behind lenses and permissions.

Press `Ctrl+C` in the TUI to quit; the server keeps running. To stop it too, `Ctrl+C` the server terminal.
