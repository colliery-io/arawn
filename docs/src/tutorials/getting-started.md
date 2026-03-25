# Getting Started with Arawn

This tutorial walks you through installing Arawn, asking your first question,
exploring the interactive interfaces, and understanding workstreams. By the end
you will have a running Arawn instance and know how to use all three interaction
modes: one-shot, REPL, and TUI.

**Time:** 15 minutes
**Prerequisites:** A terminal, an Anthropic (or OpenAI/Groq) API key

---

## Step 1: Install Arawn

The fastest path is the install script. It downloads the latest release binary
and installs sandbox dependencies on Linux (macOS needs nothing extra):

```bash
curl -fsSL https://raw.githubusercontent.com/colliery-io/arawn/main/scripts/install.sh | sh
```

The script accepts several flags if you need to customize the install:

```bash
# Install to a specific directory
./install.sh --install-dir /usr/local/bin

# Pin a version
./install.sh --version v0.1.0

# Skip sandbox dependency installation (Linux)
./install.sh --skip-deps

# Preview without making changes
./install.sh --dry-run
```

### Alternative: Build from source

If you prefer to build from source, you need Rust 1.85+ and a C compiler:

```bash
git clone https://github.com/colliery-io/arawn.git
cd arawn
cargo build --release
```

The binary lands at `./target/release/arawn`. Copy it somewhere on your `PATH`,
or run it directly.

### Verify the installation

```bash
arawn --version
```

Expected output:

```
arawn 0.1.0
```

## Step 2: Configure an LLM backend

Arawn needs an LLM to function. The simplest setup is an environment variable.
Pick whichever provider you have a key for:

```bash
# Anthropic (Claude) -- recommended
export ANTHROPIC_API_KEY="sk-ant-api03-..."

# Or OpenAI
export OPENAI_API_KEY="sk-..."

# Or Groq (fast open-source inference)
export GROQ_API_KEY="gsk_..."
```

For a permanent setup, add the export to your shell profile (`~/.zshrc`,
`~/.bashrc`, etc.) or create a configuration file at `~/.config/arawn/config.toml`:

```toml
[llm]
backend = "anthropic"
model = "claude-sonnet-4-20250514"
```

Confirm the configuration loaded correctly:

```bash
arawn config show
```

You should see your backend and model listed in the output. If the API key was
picked up from the environment, it will show as `[set via env]`.

## Step 3: Ask a one-shot question

The `arawn ask` command sends a single question and prints the response:

```bash
arawn ask "What is Rust?"
```

Expected output (abbreviated):

```
Rust is a systems programming language focused on safety, speed, and
concurrency. It achieves memory safety without a garbage collector through
its ownership system...
```

This is the simplest way to interact with Arawn. It creates a temporary session,
gets a response, and exits. No state is persisted between `ask` invocations.

## Step 4: Start an interactive chat session

For a back-and-forth conversation, use the REPL:

```bash
arawn chat
```

You will see a prompt where you can type messages:

```
arawn> What are the main components of a Rust project?

A typical Rust project contains:
- Cargo.toml — the manifest with dependencies and metadata
- src/main.rs or src/lib.rs — the entry point
- src/ — additional source modules
- tests/ — integration tests
- benches/ — benchmarks

arawn> How do I add a dependency?

You can add a dependency by editing Cargo.toml or using the cargo add command:

  cargo add serde

arawn>
```

Type `exit` or press `Ctrl+D` to leave the REPL. The session history is kept
in memory for the duration of the conversation, so the agent can refer to
earlier messages.

## Step 5: Start the server and use the TUI

Arawn has a full terminal user interface (TUI) that communicates with the server
over HTTP and WebSockets. First, start the server:

```bash
arawn start
```

Expected output:

```
[INFO] Arawn server listening on http://127.0.0.1:8080
```

The server runs in the foreground by default. Open a second terminal and launch
the TUI:

```bash
arawn tui
```

The TUI presents a multi-pane interface:

```
┌─ Sessions ──────┬─ Chat ──────────────────────────────────┐
│                  │                                         │
│  scratch         │  Welcome to Arawn.                      │
│                  │  Type a message to begin.               │
│                  │                                         │
│                  │                                         │
│                  ├─ Input ─────────────────────────────────┤
│                  │ >                                       │
└──────────────────┴─────────────────────────────────────────┘
```

Key bindings to know:

| Key | Action |
|-----|--------|
| `Tab` | Cycle focus between panes |
| `Enter` | Send message |
| `Ctrl+N` | New session |
| `Ctrl+C` / `q` | Quit |

Type a message in the input pane and press Enter. The response streams in
token-by-token. You can scroll the chat pane with the arrow keys when it has
focus.

When you are done, quit the TUI with `Ctrl+C` and stop the server:

```bash
arawn stop
```

## Step 6: Understand workstreams

By default, every conversation happens in the **scratch** workstream. Scratch is
ephemeral -- each session gets its own isolated working directory and the context
does not carry over between sessions.

For long-running projects, create a **named workstream**. Named workstreams
persist conversation history across sessions and share a working directory.
You can create and interact with workstreams through the TUI or the REST API.

In the TUI, launch with a specific workstream:

```bash
arawn tui -w "my-research-project"
```

Or create workstreams via the REST API:

```bash
curl -X POST http://localhost:8080/api/v1/workstreams \
  -H "Content-Type: application/json" \
  -d '{"name": "My Research Project"}'
```

### Scratch vs. named: when to use which

| Scenario | Use |
|----------|-----|
| Quick one-off question | `arawn ask` (no workstream) |
| Throwaway exploration | `arawn chat` (scratch) |
| Multi-day project | Named workstream via TUI (`arawn tui -w "name"`) |
| Team collaboration | Named workstream with shared ID via API |

## Step 7: Explore the data directories

Arawn stores everything locally across two directories: a configuration
directory and a data directory.

The **configuration directory** holds config files, databases, and secrets:

```
~/.config/arawn/
├── config.toml          # Your configuration (if created)
├── memory.db            # SQLite — memories, sessions, notes
├── memory.graph.db      # Knowledge graph database
├── workstreams.db       # Workstream metadata
├── identity.age         # Encryption key for secrets
├── secrets.age          # Age-encrypted secrets
└── logs/                # Daily rotating log files
```

The **data directory** holds workstream files:

```
~/.arawn/
└── workstreams/         # Working directories per workstream
```

See [File Paths & Directory Layout](../reference/file-paths.md) for the
complete directory structure.

The default embedding provider is **local ONNX** (`all-MiniLM-L6-v2`), which
runs entirely offline with zero configuration. The model files are downloaded
automatically on first use (~80 MB). This means memory and semantic search work
out of the box without any API calls for embeddings.

## Step 8: Verify everything works

Run through this checklist to confirm your setup:

```bash
# Version
arawn --version

# Config
arawn config show

# One-shot
arawn ask "Hello, are you working?"

# Server
arawn start &
arawn status
arawn stop
```

If `arawn status` reports the server as running and `arawn ask` returns a
response, you are ready to go.

---

## What you learned

- How to install Arawn via the install script or from source
- How to configure an LLM backend with an environment variable or config file
- Three ways to interact: `arawn ask` (one-shot), `arawn chat` (REPL),
  `arawn tui` (terminal UI)
- The difference between scratch and named workstreams
- Where Arawn stores its data on disk

## Next steps

- [Setting Up Memory and Knowledge](memory-setup.md) -- give Arawn persistent
  memory across sessions
- [Building Your First Workflow](first-workflow.md) -- automate tasks with the
  pipeline engine
- [Configuration Reference](../reference/configuration.md) -- full list of every
  config option
