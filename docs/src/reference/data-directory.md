# Data directory layout

*Reference. The structure of `~/.arawn/` (or whatever `--data-dir` / `ARAWN_DATA_DIR` points at).*

Source: `crates/arawn-storage/src/layout.rs::DataLayout::v1` plus `.join(` greps across crates.

## Top-level

```
<data_dir>/
├── arawn.toml                 # configuration
├── arawn.db                   # core SQLite — sessions, workstreams, feeds registry + run state, todos, ceremony state
├── memory.db                  # global knowledge-base entities, relations, FTS, vectors (all colocated)
├── projections.db             # palace projections (SQLite + sqlite-vec)
├── workflows.db               # cloacina state — workflow runs, schedules, attempts
├── settings.json              # plugin enable/disable + per-plugin user_config
│
├── workstreams/               # per-workstream data (created on workstream create)
│   ├── <name>/
│   │   ├── memory.db          # workstream-scoped knowledge base
│   │   └── workspace/         # FS-isolated working directory for shell + file tools
│   └── scratch/               # default workstream for one-off sessions
│
├── data/                      # feed-mirrored content
│   └── <provider>/<template>/<feed_id>/
│       ├── meta.json          # runtime-managed cursor + last-run status
│       └── <template files>   # JSONL / JSON / mirrored bodies
│
├── plugins/                   # plugin installation root
│   ├── installed_plugins.json # installed plugin manifest registry
│   ├── tools/                 # plugin tool dylib install dir
│   └── build/                 # plugin compilation scratch
│
├── workflows/                 # compiled workflow dylibs
│   └── <name>/
│       ├── package.toml
│       └── lib<name>.{dylib,so}
│
├── prompts/                   # custom user prompts
│
├── agents/                    # user-defined agent types (markdown + frontmatter)
│
├── integrations/              # encrypted credential blobs per integration
│   └── <service>/
│       └── <service>.bin      # encrypted (ChaCha20Poly1305)
│
├── tokens/                    # OAuth tokens (encrypted on disk)
│
├── models/                    # ML model files for memory/embed
│   └── all-MiniLM-L6-v2/
│       └── model.onnx
│
└── logs/                      # daily-rolling logs
    ├── server.log
    └── tui.log
```

## What gets created when

The `DataLayout::v1` reconciler eagerly creates `workstreams/`, `plugins/tools/`, `plugins/build/`, and `prompts/` on first startup. The other directories appear lazily as their owning subsystem first writes (e.g. `tokens/` appears on first `/connect`, `data/` on first feed run, `models/` if you've installed the embedder).

## What's safe to delete

| Path | Safe to delete? | Effect |
|---|---|---|
| `arawn.toml` | yes (regenerates with defaults) | Loses your config — provider keys, integrations, permission rules. |
| `arawn.db` | NO while server is running | Loses all sessions, workstream metadata, feed registry + run state, todos. Server must be stopped first. |
| `memory.db` | yes | Wipes global knowledge base (entities + graph + FTS + vectors are all in this one file). |
| `projections.db` | yes (will rebuild from feed data on next extraction run) | Wipes palace state. |
| `workflows.db` | NO while server is running | Loses workflow schedule state. |
| `data/<provider>/...` | yes (will re-mirror) | Cron-driven re-fetch from `since=` cursor in `meta.json` if present. |
| `data/<provider>/<feed>/meta.json` | yes | Backfill from scratch on next feed run. |
| `plugins/` | yes (uninstalls all plugins) | Plugin reinstall needed. |
| `integrations/<svc>/<svc>.bin` | yes | `/disconnect <svc>` equivalent — need to `/connect` again. |
| `tokens/` | yes | All OAuth tokens dropped; re-`/connect` for each service. |
| `models/all-MiniLM-L6-v2/` | yes (degrades to FTS-only) | Memory falls back to keyword search until you reinstall the model. |
| `logs/` | yes | Loses logs. |

## Workstream FS isolation

The `workstreams/<name>/workspace/` directory is the sandbox root for shell + file tools when that workstream is active. Writes outside `workspace/` are blocked by the shell sandbox (see [shell sandbox reference](./shell-sandbox.md)).

`workstreams/scratch/` is the default workstream — created idempotently on startup. Sessions that haven't been promoted to a named workstream live here.

## Encrypted blobs

Two locations hold encrypted-at-rest data:

- `integrations/<service>/<service>.bin` — provider-specific tokens (Slack workspace token, Atlassian cloud_id + token, GitHub installation_id).
- `tokens/` — OAuth2 refresh tokens for the OAuth-flow providers (Google, Slack, Atlassian).

Both use ChaCha20Poly1305 with a key derived from the user's keyring (or a fallback file under `<data_dir>` when the keyring isn't available).
