# Building Your First Workflow

This tutorial walks you through the Arawn pipeline engine: enabling it,
writing a workflow definition in TOML, understanding WASM runtimes, running
the workflow, and scheduling it with cron. By the end you will have a working
automated workflow that fetches data, transforms it, and writes the result to
a file.

**Time:** 25 minutes
**Prerequisites:** A working Arawn installation ([Getting Started](getting-started.md))

---

## What is the pipeline engine?

The pipeline engine lets you define multi-step workflows as TOML files. Each
step (task) runs inside a sandboxed WASM runtime, and tasks can depend on each
other to form a directed acyclic graph (DAG). The engine handles execution
order, retries, timeouts, and scheduling.

Key concepts:

| Concept | Description |
|---------|-------------|
| **Workflow** | A TOML file defining a sequence of tasks |
| **Task** | A single step that runs in a WASM runtime |
| **Runtime** | A compiled WASM module (shell, http, file_read, file_write, transform, passthrough) |
| **Capabilities** | Per-task WASI permission grants (filesystem, network, memory) |
| **Schedule** | Optional cron expression for recurring execution |

## Step 1: Enable the pipeline engine

Open your configuration file:

```bash
${EDITOR:-nano} ~/.config/arawn/config.toml
```

Add or update the pipeline section:

```toml
[pipeline]
enabled = true
database = "pipeline.db"
workflow_dir = "workflows"
max_concurrent_tasks = 4
task_timeout_secs = 300
pipeline_timeout_secs = 600
cron_enabled = true
```

| Setting | Value | Purpose |
|---------|-------|---------|
| `enabled` | `true` | Turns on the pipeline engine |
| `database` | `"pipeline.db"` | SQLite file for pipeline state (relative to data dir) |
| `workflow_dir` | `"workflows"` | Directory where workflow TOML files live (relative to data dir) |
| `max_concurrent_tasks` | `4` | How many tasks can run in parallel |
| `task_timeout_secs` | `300` | Per-task timeout (5 minutes) |
| `pipeline_timeout_secs` | `600` | Whole-pipeline timeout (10 minutes) |
| `cron_enabled` | `true` | Enables cron-based scheduling |

## Step 2: Create the workflow directory

Create the directory where workflow definitions will live:

```bash
mkdir -p ~/.config/arawn/workflows
```

## Step 3: Understand the available runtimes

Arawn ships with six built-in WASM runtimes in the `runtimes/` directory. Each
is compiled to `wasm32-wasip1` and cached by SHA-256 content hash.

| Runtime | Purpose | Key config fields |
|---------|---------|-------------------|
| `shell` | Execute a shell command | `command`, `args`, `stdin` |
| `http` | Make an HTTP request | `url`, `method`, `headers`, `body` |
| `file_read` | Read a file from disk | `path` |
| `file_write` | Write content to a file | `path`, `content` |
| `transform` | Transform data with a template | `template`, `input` |
| `passthrough` | Pass input to output unchanged | *(none)* |

Runtimes receive JSON input on stdin and return JSON output on stdout:

```json
// Input format
{
  "config": { /* task-specific config from the TOML */ },
  "context": { /* output from upstream tasks */ }
}

// Output format
{
  "status": "success",
  "output": { /* result data */ }
}
```

## Step 4: Write your first workflow

Create a workflow that fetches a webpage, extracts a summary line, and writes it
to a file. This exercises three runtimes: `http`, `transform`, and `file_write`.

Create the file `~/.config/arawn/workflows/daily_summary.toml`:

```bash
cat > ~/.config/arawn/workflows/daily_summary.toml << 'TOML'
[workflow]
name = "daily_summary"
description = "Fetch a webpage and save a summary to a local file"

# Task 1: Fetch the page
[[workflow.tasks]]
id = "fetch_page"
runtime = "http"
config = { url = "https://httpbin.org/get", method = "GET" }
retry_attempts = 2
retry_delay_ms = 1000

# Task 2: Extract the interesting parts
[[workflow.tasks]]
id = "extract_info"
runtime = "transform"
config = { template = "Fetched from: {{context.fetch_page.output.url}}\nOrigin: {{context.fetch_page.output.origin}}" }
dependencies = ["fetch_page"]

# Task 3: Write the result to a file
[[workflow.tasks]]
id = "save_result"
runtime = "file_write"
config = { path = "/tmp/arawn-daily-summary.txt" }
dependencies = ["extract_info"]

[workflow.capabilities]
filesystem = ["/tmp/arawn-*"]
network = true

[workflow.runtime]
timeout_secs = 60
max_retries = 1
TOML
```

Let us break down the structure:

### Workflow header

```toml
[workflow]
name = "daily_summary"
description = "Fetch a webpage and save a summary to a local file"
```

Every workflow needs a unique `name`. The `description` is optional but
recommended.

### Tasks

Each `[[workflow.tasks]]` block defines one step:

```toml
[[workflow.tasks]]
id = "fetch_page"           # Unique ID within this workflow
runtime = "http"             # Which WASM runtime to use
config = { ... }            # Runtime-specific configuration
retry_attempts = 2           # Retry up to 2 times on failure
retry_delay_ms = 1000        # Wait 1 second between retries
```

### Dependencies

The `dependencies` array controls execution order:

```toml
dependencies = ["fetch_page"]
```

This means `extract_info` will not run until `fetch_page` completes
successfully. The output from `fetch_page` is available in the context as
`context.fetch_page.output`.

The execution graph for our workflow looks like this:

```
fetch_page ──> extract_info ──> save_result
```

Tasks without dependencies run immediately. Tasks with multiple dependencies
wait for all of them.

### Capabilities

WASI capabilities grant sandboxed permissions to the workflow:

```toml
[workflow.capabilities]
filesystem = ["/tmp/arawn-*"]    # Allow writes to matching paths
network = true                    # Allow outbound HTTP
```

Without explicit grants, tasks cannot access the filesystem or network. This is
a security feature -- workflows run in a sandbox by default.

## Step 5: Validate the workflow

Before running, make sure the TOML parses and the runtimes are available. Use
the `catalog` tool to inspect what is registered:

```bash
arawn ask "Use the catalog tool to list all registered runtimes"
```

Expected output includes the six built-in runtimes: `shell`, `http`,
`file_read`, `file_write`, `transform`, `passthrough`.

You can also inspect a specific runtime:

```bash
arawn ask "Use the catalog tool to inspect the http runtime"
```

## Step 6: Run the workflow

Execute the workflow:

```bash
arawn ask "Use the workflow tool to run the daily_summary workflow"
```

The agent will:

1. Load `daily_summary.toml` from the workflow directory
2. Compile any WASM runtimes that are not already cached
3. Execute `fetch_page` (HTTP GET to httpbin.org)
4. Pass the result to `extract_info` (template substitution)
5. Pass the transformed text to `save_result` (write to `/tmp/`)
6. Report the result

Expected response:

```
Workflow "daily_summary" completed successfully.

Task results:
  fetch_page:    success (HTTP 200)
  extract_info:  success (transformed output)
  save_result:   success (wrote to /tmp/arawn-daily-summary.txt)
```

Verify the output file:

```bash
cat /tmp/arawn-daily-summary.txt
```

```
Fetched from: https://httpbin.org/get
Origin: 203.0.113.42
```

## Step 7: Check workflow status

List all workflows and their run history:

```bash
arawn ask "Use the workflow tool to list all workflows"
```

Check the status of a specific run:

```bash
arawn ask "Use the workflow tool to show status of the daily_summary workflow"
```

This shows the last run time, status, and per-task results.

## Step 8: Add a shell task

Let us extend the workflow with a shell task that timestamps the output. Edit
`~/.config/arawn/workflows/daily_summary.toml` and add a new task before
`save_result`:

```toml
# Task 2.5: Add a timestamp
[[workflow.tasks]]
id = "add_timestamp"
runtime = "shell"
config = { command = "date", args = ["+%Y-%m-%d %H:%M:%S"] }
dependencies = ["fetch_page"]
```

Update the `extract_info` task to use both upstream outputs:

```toml
[[workflow.tasks]]
id = "extract_info"
runtime = "transform"
config = { template = "Report generated: {{context.add_timestamp.output.stdout}}\nFetched from: {{context.fetch_page.output.url}}\nOrigin: {{context.fetch_page.output.origin}}" }
dependencies = ["fetch_page", "add_timestamp"]
```

The execution graph now looks like:

```
fetch_page ──────┐
                 ├──> extract_info ──> save_result
add_timestamp ───┘
```

`fetch_page` and `add_timestamp` run in parallel because they have no
dependency on each other. `extract_info` waits for both.

Run the workflow again to see the updated output.

## Step 9: Schedule with cron

Add a schedule to your workflow so it runs automatically. Edit the TOML and add
a schedule section:

```toml
[workflow.schedule]
cron = "0 9 * * *"
timezone = "America/New_York"
```

This runs the workflow every day at 9:00 AM Eastern. The cron syntax follows
the standard five-field format:

```
┌───────── minute (0-59)
│ ┌─────── hour (0-23)
│ │ ┌───── day of month (1-31)
│ │ │ ┌─── month (1-12)
│ │ │ │ ┌─ day of week (0-7, 0 and 7 = Sunday)
│ │ │ │ │
0 9 * * *
```

Make sure `cron_enabled = true` in your `[pipeline]` config (we set this in
Step 1).

You can also schedule a workflow through the agent:

```bash
arawn ask "Use the workflow tool to schedule daily_summary with cron '0 9 * * *'"
```

## Step 10: Write a workflow with event triggers

Instead of (or in addition to) cron, workflows can trigger on events. Add a
trigger section:

```toml
[workflow.triggers]
on_event = "session_close"
```

This runs the workflow every time a session ends. This is how the built-in
session indexing pipeline works -- it triggers on `session_close` to extract
facts and entities.

## The complete workflow file

Here is the final version of `daily_summary.toml` with all the pieces:

```toml
[workflow]
name = "daily_summary"
description = "Fetch a webpage, timestamp it, and save a summary"

[[workflow.tasks]]
id = "fetch_page"
runtime = "http"
config = { url = "https://httpbin.org/get", method = "GET" }
retry_attempts = 2
retry_delay_ms = 1000

[[workflow.tasks]]
id = "add_timestamp"
runtime = "shell"
config = { command = "date", args = ["+%Y-%m-%d %H:%M:%S"] }

[[workflow.tasks]]
id = "extract_info"
runtime = "transform"
config = { template = "Report generated: {{context.add_timestamp.output.stdout}}\nFetched from: {{context.fetch_page.output.url}}\nOrigin: {{context.fetch_page.output.origin}}" }
dependencies = ["fetch_page", "add_timestamp"]

[[workflow.tasks]]
id = "save_result"
runtime = "file_write"
config = { path = "/tmp/arawn-daily-summary.txt" }
dependencies = ["extract_info"]

[workflow.capabilities]
filesystem = ["/tmp/arawn-*"]
network = true

[workflow.runtime]
timeout_secs = 60
max_retries = 1

[workflow.schedule]
cron = "0 9 * * *"
timezone = "America/New_York"
```

---

## What you learned

- How to enable and configure the pipeline engine
- The six built-in WASM runtimes and what each one does
- How to write a multi-task workflow in TOML with dependencies
- How tasks form a DAG and execute in parallel where possible
- How WASI capabilities sandbox filesystem and network access
- How to run, inspect, and troubleshoot workflows
- How to schedule workflows with cron expressions
- How to trigger workflows on events like session close

## Next steps

- [Creating Your First Plugin](first-plugin.md) -- extend Arawn with custom
  skills and hooks
- [Configuration Reference](../reference/configuration.md) -- full pipeline
  config options
- [Built-in Tools](../reference/tools.md) -- the `catalog` and `workflow` tools
  in detail
