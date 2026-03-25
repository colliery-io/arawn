# Manage Workstreams

This guide covers creating, organizing, and maintaining workstreams -- persistent conversation contexts that span multiple sessions and preserve your agent's working memory across time.

## Prerequisites

- Arawn installed and running (either `arawn start` or using the TUI)
- Familiarity with basic Arawn chat usage

## Understand workstream types

Arawn has two kinds of workstreams:

- **Scratch** -- The default, ephemeral workspace. Every session that does not specify a workstream lands here. Each scratch session gets its own isolated directory.
- **Named** -- A persistent workspace tied to a project or topic. All sessions within a named workstream share the same production and work directories.

### Directory structure

```text
~/.arawn/workstreams/
├── scratch/
│   └── sessions/
│       ├── <session-id-1>/work/    # Isolated per session
│       └── <session-id-2>/work/
├── my-project/
│   ├── production/                 # Shared deliverables
│   └── work/                       # Shared working area
└── backend-api/
    ├── production/
    └── work/
```

Named workstreams have a `production/` directory for final artifacts and a `work/` directory for in-progress files. Scratch sessions are isolated -- each session only sees its own `work/` subdirectory.

## Create a workstream

### From the TUI

1. Open the TUI: `arawn tui`
2. In the sidebar, select "New Workstream" or press the create shortcut
3. Type a name and press Enter

### From the REST API

```bash
curl -X POST http://localhost:8080/api/v1/workstreams \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Backend API Redesign",
    "default_model": "claude-sonnet-4-20250514",
    "tags": ["backend", "q2"]
  }'
```

The response includes the workstream ID you will use in subsequent operations.

### From the TUI or REST API

Workstream management is available through the TUI and REST API. Use the TUI sidebar to create workstreams interactively, or use the REST API as shown above. To launch directly into a specific workstream:

```bash
arawn tui -w "Backend API Redesign"
```

You can also set a default workstream for a context so every session opens in that workstream automatically:

```bash
arawn config set-context dev --server http://localhost:8080 --workstream "Backend API Redesign"
```

## Switch between workstreams

### In the TUI

Use the sidebar to browse workstreams and click or navigate to the one you want. The sidebar shows all named workstreams and the scratch workstream.

### Launch directly into a workstream

```bash
arawn tui -w "Backend API Redesign"
```

### Via config context

Set a default workstream in your project-level config so every session opens in that context:

```toml
[context]
workstream = "Backend API Redesign"
```

## List workstreams

### REST API

```bash
curl http://localhost:8080/api/v1/workstreams
```

The TUI sidebar also displays all workstreams with session counts and last-used timestamps.

## Promote scratch to a named workstream

When a scratch session grows into a real project, promote it:

### REST API

```bash
curl -X POST http://localhost:8080/api/v1/workstreams/{id}/promote \
  -H "Content-Type: application/json" \
  -d '{
    "title": "New Project Name",
    "tags": ["research"]
  }'
```

Promotion moves the conversation history and files from the scratch session into a new named workstream. The scratch workstream resets to empty afterward.

## Work with files

Workstreams provide structured file operations through the directory manager.

### Promote a file from work to production

Move a file from the `work/` directory to `production/` when it is ready:

```bash
curl -X POST http://localhost:8080/api/v1/workstreams/{id}/files/promote \
  -H "Content-Type: application/json" \
  -d '{"path": "draft-report.md"}'
```

The file is copied to `production/` and, if a name conflict exists, it is automatically renamed.

### Export a file to an external location

```bash
curl -X POST http://localhost:8080/api/v1/workstreams/{id}/files/export \
  -H "Content-Type: application/json" \
  -d '{"path": "report.md", "destination": "/home/user/documents/"}'
```

### Clone a repository into the work directory

```bash
curl -X POST http://localhost:8080/api/v1/workstreams/{id}/clone \
  -H "Content-Type: application/json" \
  -d '{"url": "https://github.com/org/repo.git"}'
```

The repository is cloned into the workstream's `work/` directory.

## Monitor disk usage

### Check usage for a single workstream

```bash
curl http://localhost:8080/api/v1/workstreams/{id}/usage
```

Returns production and work directory sizes, per-session breakdowns (for scratch), and any threshold warnings.

### Configure usage thresholds

Set warning thresholds so Arawn alerts you before disk usage grows too large:

```toml
[paths.usage]
total_warning_gb = 10
workstream_warning_gb = 1
session_warning_mb = 200
```

| Setting | Default | Description |
|---------|---------|-------------|
| `total_warning_gb` | 10 | Warn when total Arawn data exceeds this size |
| `workstream_warning_gb` | 1 | Warn when a single workstream exceeds this size |
| `session_warning_mb` | 200 | Warn when a single session exceeds this size |

## Clean up old data

### Automatic scratch cleanup

Scratch sessions older than a configured threshold are cleaned up automatically:

```toml
[paths.cleanup]
scratch_cleanup_days = 7
dry_run = false
```

Set `dry_run = true` to log what would be deleted without actually removing anything.

### Manual cleanup

Trigger cleanup for a specific workstream via the API:

```bash
curl -X POST http://localhost:8080/api/v1/workstreams/{id}/cleanup
```

This removes temporary files and old session data from the `work/` directory. If more than 100 files would be deleted, the response asks for confirmation.

### Dry run

Add the `dry_run` query parameter to preview what would be deleted:

```bash
curl -X POST "http://localhost:8080/api/v1/workstreams/{id}/cleanup?dry_run=true"
```

## Archive a workstream

When a project is finished, archive it to remove it from active listings while preserving data:

```bash
curl -X POST http://localhost:8080/api/v1/workstreams/{id}/archive
```

Archived workstreams do not appear in default listings but remain on disk. To include them:

```bash
curl "http://localhost:8080/api/v1/workstreams?include_archived=true"
```

## Further reading

- [Configuration Reference](../reference/configuration.md) -- full list of
  workstream, paths, usage, cleanup, and monitoring settings
- [Sessions & Workstreams](../explanation/sessions-and-workstreams.md) -- design
  rationale for the session and workstream model
