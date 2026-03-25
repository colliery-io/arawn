# Creating Your First Plugin

This tutorial walks you through building an Arawn plugin from scratch. You will
create a plugin manifest, write a skill, define a hook that validates shell
commands, and test the whole thing. By the end you will have a working plugin
that extends Arawn with custom behavior.

**Time:** 20 minutes
**Prerequisites:** A working Arawn installation ([Getting Started](getting-started.md)),
basic familiarity with JSON and Markdown

---

## What is a plugin?

A plugin is a directory that bundles one or more extensions for Arawn:

- **Skills** -- reusable prompt templates that give the agent specialized
  behavior (e.g., a code review skill, a research skill)
- **Agents** -- subagent configurations with their own model, tools, and
  system prompt
- **Hooks** -- event handlers that intercept tool calls, react to session
  events, or audit agent behavior
- **CLI tools** -- external executables exposed to the agent as callable tools

Plugins follow the Claude Code plugin format, making them compatible across
tools in the ecosystem.

## Step 1: Create the plugin directory

Plugins live in either of two locations:

| Location | Scope |
|----------|-------|
| `~/.config/arawn/plugins/` | User-level, available in all projects |
| `./plugins/` | Project-local, only available in that directory |

For this tutorial, we will create a user-level plugin:

```bash
mkdir -p ~/.config/arawn/plugins/my-first-plugin/.claude-plugin
mkdir -p ~/.config/arawn/plugins/my-first-plugin/skills/reviewer
mkdir -p ~/.config/arawn/plugins/my-first-plugin/agents
mkdir -p ~/.config/arawn/plugins/my-first-plugin/hooks
```

Your directory structure now looks like:

```
~/.config/arawn/plugins/my-first-plugin/
├── .claude-plugin/
├── skills/
│   └── reviewer/
├── agents/
└── hooks/
```

## Step 2: Write the plugin manifest

The manifest at `.claude-plugin/plugin.json` declares what the plugin provides.
Create it:

```bash
cat > ~/.config/arawn/plugins/my-first-plugin/.claude-plugin/plugin.json << 'EOF'
{
  "name": "my-first-plugin",
  "version": "1.0.0",
  "description": "A tutorial plugin with a code review skill and a shell safety hook",
  "author": {
    "name": "Your Name"
  },
  "skills": "./skills/",
  "agents": "./agents/",
  "hooks": "./hooks/hooks.json"
}
EOF
```

The `name` field is required and must be kebab-case. The other fields (`version`,
`description`, `author`, `skills`, `agents`, `hooks`, `commands`) are optional.
All paths are relative to the plugin root directory (where `.claude-plugin/` lives).

> For the full list of manifest fields and validation rules, see the
> [Plugin Manifest Reference](../reference/plugin-manifest.md).

## Step 3: Create a skill

A skill is a Markdown file with YAML frontmatter that provides the agent with
specialized instructions. Create the code review skill:

```bash
cat > ~/.config/arawn/plugins/my-first-plugin/skills/reviewer/SKILL.md << 'EOF'
---
name: code-reviewer
description: Review code for bugs, security issues, and style problems
---

You are a code review specialist. When asked to review code, follow this
process:

## Step 1: Understand the context

Read the file or diff provided. Identify the programming language and
the purpose of the code.

## Step 2: Check for issues

Analyze the code for:

- **Security vulnerabilities** -- injection, exposed secrets, unsafe operations
- **Bugs** -- off-by-one errors, null/None handling, race conditions
- **Performance** -- unnecessary allocations, O(n^2) loops, missing caching
- **Style** -- naming conventions, dead code, overly complex logic

## Step 3: Provide feedback

For each issue found, report:

1. **File and line** -- where the issue is
2. **Severity** -- critical, warning, or suggestion
3. **Description** -- what the problem is
4. **Fix** -- a concrete code change that resolves it

If the code looks good, say so. Do not invent problems.

## Output format

Use this structure:

```
## Review: <filename>

### Critical
- Line 42: SQL injection via unsanitized input. Use parameterized queries.

### Warnings
- Line 17: Unwrap on a Result that can fail in production. Use `?` or match.

### Suggestions
- Line 5: Consider renaming `x` to `connection_count` for clarity.

### Summary
<1-2 sentence overall assessment>
```
EOF
```

The YAML frontmatter must include `name` and `description`. The body is standard
Markdown that gets injected into the agent's prompt when the skill is activated.

### Skill naming rules

- `name` must be unique across all loaded plugins
- Use kebab-case: `code-reviewer`, not `Code Reviewer`
- The directory name (`reviewer/`) does not need to match the skill name, but
  it helps for organization

## Step 4: Create an agent definition

Agents are subagent configurations -- they define a specialized agent that the
main agent can delegate to. Create one:

```bash
cat > ~/.config/arawn/plugins/my-first-plugin/agents/code-review.md << 'EOF'
---
name: code-review-agent
description: Delegated code review agent with file access
model: claude-sonnet
tools:
  - file_read
  - grep
  - glob
max_iterations: 15
---

You are a dedicated code review agent. You have been delegated a review task
by the primary agent.

Your job:
1. Read the files specified in the task
2. Analyze them for bugs, security issues, and style problems
3. Return a structured review report

Use the file_read tool to access source files. Use grep and glob to find
related code if you need more context.

Be thorough but concise. Focus on actionable findings.
EOF
```

The frontmatter defines:

| Field | Purpose |
|-------|---------|
| `name` | Unique agent identifier |
| `description` | Shown when the main agent considers delegation |
| `model` | LLM model to use (references an `[llm.<name>]` profile) |
| `tools` | Allowed tools (the agent cannot use tools not listed here) |
| `max_iterations` | Safety limit on tool loop iterations |

The body is the agent's system prompt.

## Step 5: Create a hook

Hooks intercept events in the agent lifecycle. We will create a `PreToolUse`
hook that blocks dangerous shell commands.

First, create the hook configuration:

```bash
cat > ~/.config/arawn/plugins/my-first-plugin/hooks/hooks.json << 'EOF'
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "shell",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/hooks/validate-shell.sh",
            "timeout": 5000
          }
        ]
      }
    ],
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/hooks/session-hello.sh"
          }
        ]
      }
    ]
  }
}
EOF
```

This registers two hooks:

1. **PreToolUse** on the `shell` tool -- runs `validate-shell.sh` before every
   shell command. If the script exits non-zero, the command is blocked.
2. **SessionStart** -- runs `session-hello.sh` when a session begins.

The `${CLAUDE_PLUGIN_ROOT}` variable expands to the plugin's root directory at
runtime, so paths work regardless of where the plugin is installed.

### Write the shell validator

```bash
cat > ~/.config/arawn/plugins/my-first-plugin/hooks/validate-shell.sh << 'SCRIPT'
#!/bin/bash
# Validate shell commands before execution.
# Receives JSON on stdin: {"tool": "shell", "params": {"command": "..."}}
# Exit 0 = allow, exit non-zero = block (stdout becomes the block reason).

input=$(cat)
command=$(echo "$input" | jq -r '.params.command // .params.cmd // ""')

# Block commands that delete recursively from root or home
if echo "$command" | grep -qE 'rm\s+-rf\s+(/|~|\$HOME)'; then
  echo "Blocked: recursive delete of root or home directory"
  exit 1
fi

# Block disk-wiping commands
if echo "$command" | grep -qE '(dd\s+if=|mkfs\.|wipefs)'; then
  echo "Blocked: disk-wiping command detected"
  exit 1
fi

# Block curl piped to shell (common attack vector)
if echo "$command" | grep -qE 'curl.*\|\s*(bash|sh|zsh)'; then
  echo "Blocked: piping curl output to a shell is not allowed"
  exit 1
fi

# Allow everything else
exit 0
SCRIPT

chmod +x ~/.config/arawn/plugins/my-first-plugin/hooks/validate-shell.sh
```

### Write the session greeting

```bash
cat > ~/.config/arawn/plugins/my-first-plugin/hooks/session-hello.sh << 'SCRIPT'
#!/bin/bash
# Runs at session start. Stdout is returned as informational output.
# Receives JSON on stdin: {"session_id": "..."}

cat > /dev/null  # consume stdin
echo "my-first-plugin loaded. Shell safety checks active."
SCRIPT

chmod +x ~/.config/arawn/plugins/my-first-plugin/hooks/session-hello.sh
```

## Step 6: Review the final directory structure

Your plugin should now look like this:

```
~/.config/arawn/plugins/my-first-plugin/
├── .claude-plugin/
│   └── plugin.json
├── skills/
│   └── reviewer/
│       └── SKILL.md
├── agents/
│   └── code-review.md
└── hooks/
    ├── hooks.json
    ├── validate-shell.sh
    └── session-hello.sh
```

## Step 7: Verify the plugin loads

Make sure the plugin directory is configured. Check your `~/.config/arawn/config.toml`:

```toml
[plugins]
enabled = true
dirs = ["~/.config/arawn/plugins"]
hot_reload = true
```

Now verify Arawn sees the plugin:

```bash
arawn plugin list
```

Expected output:

```
Installed plugins:

  my-first-plugin  v1.0.0
    A tutorial plugin with a code review skill and a shell safety hook
    Skills:  code-reviewer
    Agents:  code-review-agent
    Hooks:   PreToolUse (shell), SessionStart
```

If the plugin does not appear, check that:
- The `.claude-plugin/plugin.json` file exists and is valid JSON
- The `name` field uses kebab-case and starts with a letter
- The paths in the manifest point to directories that exist

## Step 8: Test the skill

Start a chat and ask the agent to use the code review skill:

```bash
arawn chat
```

```
arawn> Review this Rust function for issues:
       fn get_user(id: &str) -> User {
           let query = format!("SELECT * FROM users WHERE id = '{}'", id);
           db.execute(&query).unwrap().first().unwrap().into()
       }

## Review: inline code

### Critical
- Line 2: SQL injection vulnerability. The `id` parameter is interpolated
  directly into the SQL string. Use parameterized queries instead:
  `db.execute("SELECT * FROM users WHERE id = ?", &[id])`

### Warnings
- Line 3: Double unwrap on database result. If the query fails or returns
  no rows, this will panic. Use `?` or return an `Option<User>`.

### Summary
This function has a critical SQL injection vulnerability and will panic
on missing users. Both issues need to be fixed before production use.
```

The agent used the `code-reviewer` skill's instructions to structure its review.

## Step 9: Test the hook

Try a command that should be blocked:

```bash
arawn chat
```

```
arawn> Run the command: rm -rf /

I attempted to run that command, but it was blocked by a safety hook:
"Blocked: recursive delete of root or home directory"

This is a destructive command that would delete your entire filesystem.
I cannot execute it.
```

The `PreToolUse` hook intercepted the shell tool call, detected the dangerous
pattern, and returned a non-zero exit code with a reason. The agent received the
block message and reported it to you.

Now try a safe command:

```
arawn> Run: echo "hello from the plugin tutorial"

hello from the plugin tutorial
```

Safe commands pass through the hook validator and execute normally.

## Step 10: Enable hot reload

With `hot_reload = true` in your plugin config, Arawn watches the plugin
directories for changes. When you edit a skill, hook, or manifest, the changes
take effect without restarting the server.

Try it:

1. Start the server: `arawn start`
2. Edit the skill at `~/.config/arawn/plugins/my-first-plugin/skills/reviewer/SKILL.md`
3. Change the review instructions
4. The next conversation automatically uses the updated skill

This makes development fast -- edit, save, test.

---

## What you learned

- The anatomy of an Arawn plugin: manifest, skills, agents, and hooks
- How to write a skill with YAML frontmatter and Markdown instructions
- How to define a subagent with restricted tools and a focused system prompt
- How to create PreToolUse hooks that can block dangerous operations
- How to create SessionStart hooks for initialization
- How to test plugins interactively and verify hook behavior
- How hot reload speeds up the development cycle

## Next steps

- [Plugin Manifest Reference](../reference/plugin-manifest.md) -- full plugin
  format and manifest documentation
- [Plugins & Hooks](../explanation/plugins-and-hooks.md) -- how the plugin
  system extends Arawn with skills, agents, and lifecycle hooks
- [Use Subagents and Delegation](../how-to/use-subagents.md) -- how the main
  agent delegates to subagents
- [Configuration Reference](../reference/configuration.md) -- full plugin
  config options
