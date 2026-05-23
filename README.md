# Arawn

A self-hosted personal agentic assistant in a single Rust binary. Runs a local
WebSocket server, talks to any OpenAI-compatible LLM (Groq, Ollama Cloud,
local Ollama, OpenAI), and exposes a TUI for interactive sessions plus a
workflow runtime for scheduled background jobs.

> **Status: alpha.** Breaking changes expected. Suitable for hacking on, not
> for putting in front of users yet.

## 30-second quickstart

```sh
# 1. Build
cargo build --release

# 2. Configure (you can do this manually or wait for `arawn init` — see T-0194)
mkdir -p ~/.arawn
cat > ~/.arawn/arawn.toml <<'EOF'
[llm.default]
provider = "groq"
model = "openai/gpt-oss-120b"
api_key_env = "GROQ_API_KEY"

[engine]
llm = "default"
EOF

# 3. Set the API key
export GROQ_API_KEY=gsk_…

# 4. Run the server
./target/release/arawn serve

# 5. In another terminal, open the TUI
./target/release/arawn tui
```

Type a message in the TUI and arawn will respond using the configured model.
The agent has tools for shell, file editing, web search, and more.

For Ollama Cloud (`OLLAMA_API_KEY` instead, model like `gemma4:31b-cloud`,
provider `https://ollama.com/v1`) and the full troubleshooting walkthrough,
see **[docs/src/getting-started.md](docs/src/getting-started.md)**.

## What's in here

- `crates/arawn/` — the binary, CLI, server entry points
- `crates/arawn-engine/` — agentic loop, tool registry, permissions, plugins
- `crates/arawn-llm/` — provider-agnostic LLM client (`OpenAICompatibleClient`,
  retry, warmup caching)
- `crates/arawn-tui/` — terminal UI client
- `crates/arawn-memory/` — persistent knowledge base (global + per-workstream)
- `crates/arawn-workflow/` — scheduled DAG pipelines (cron + on-demand)
- `crates/arawn-tests/` — integration + UAT (LLM-as-judge) tests
- `.metis/` — work tracking (vision, initiatives, tasks). See `.metis/vision.md`.

## CLI

```
arawn                       # one-shot prompt (uses running server)
arawn serve                 # start the WebSocket server
arawn tui                   # launch the TUI client
arawn plugin <subcommand>   # plugin management
arawn --list-sessions       # list resumable sessions
arawn --session <uuid>      # resume a specific session
```

`--data-dir <path>` (or `ARAWN_DATA_DIR`) overrides the data directory
(default: `~/.arawn`).

## Development

Tasks are run via [angreal](https://github.com/colliery-io/angreal):

```
angreal build workspace        # cargo build
angreal test unit              # workspace lib tests
angreal test integration       # ignored integration tests
angreal test uat               # end-to-end against a real LLM
angreal check all              # fmt + clippy + cargo check
angreal docs serve             # serve docs locally (mdbook)
```

Code map and semantic summaries live in
[`.metis/code-index.md`](.metis/code-index.md). Work tracking is in
`.metis/` — see [active initiatives](.metis/initiatives/) and
[backlog tasks](.metis/backlog/).

## Vision

> A lightweight, self-hosted personal agentic assistant that runs scheduled
> tasks, monitors channels (email, GitHub, etc.), surfaces action items, and
> provides an interactive chat interface — all from a single Rust binary with
> a minimal resource footprint.

Full vision: [`.metis/vision.md`](.metis/vision.md).
