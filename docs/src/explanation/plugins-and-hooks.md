# Plugins & Hooks

Arawn's plugin system lets you extend the agent with new skills, custom agents, and lifecycle hooks without modifying the core codebase. This page explains how the plugin system works, why it is designed the way it is, and what the security implications are.

## Why Plugins?

Arawn is a general-purpose agent, but every user has specific needs. A security researcher needs tools for vulnerability analysis. A data scientist needs tools for dataset exploration. A DevOps engineer needs tools for infrastructure management.

Rather than building every possible capability into the core binary, Arawn provides an extension point. Plugins add domain-specific skills, specialized agent configurations, and lifecycle hooks that customize behavior. This keeps the core focused while allowing unbounded specialization.

## Plugin Format

Arawn uses the Claude Code plugin format. This is a deliberate compatibility choice: plugins written for Claude Code work with Arawn, and vice versa. The format is straightforward:

```
my-plugin/
  .claude-plugin/
    plugin.json          # Manifest
  skills/
    my-skill/
      SKILL.md           # Skill definition (YAML frontmatter + markdown body)
  agents/
    my-agent.md          # Agent definition (YAML frontmatter + markdown body)
  hooks/
    hooks.json           # Hook definitions
  scripts/
    on-end.sh            # Hook scripts
```

### The Manifest

`plugin.json` declares the plugin's metadata and component locations:

```json
{
  "name": "my-plugin",
  "version": "1.0.0",
  "description": "A useful plugin",
  "skills": "./skills/",
  "agents": "./agents/",
  "hooks": "./hooks/hooks.json"
}
```

All paths in the manifest are relative to the plugin root directory. The `CLAUDE_PLUGIN_ROOT` environment variable is set to the plugin's directory when hook scripts execute, making paths portable.

### Why This Format?

Choosing a plugin format is a decision with long-term consequences. A custom format would be more tailored to Arawn's specific needs, but it would mean zero existing plugins at launch. By adopting the Claude Code format, Arawn inherits an existing ecosystem of skills, agents, and hooks. Plugin authors can target both systems with the same files.

The format is also minimal. A plugin is a directory with a JSON manifest and markdown files. There are no compiled artifacts, no build steps, no binary dependencies. This lowers the barrier to entry: anyone who can write a markdown file can write a plugin.

## Plugin Discovery

`PluginManager` scans two directories for plugins:

1. **User-level**: `~/.config/arawn/plugins/` -- shared across all projects.
2. **Project-level**: `./plugins/` -- specific to the current working directory.

Each subdirectory that contains `.claude-plugin/plugin.json` is treated as a plugin. The manager iterates through subdirectories, checks for the manifest, and attempts to load each one. Invalid plugins (bad JSON, missing files, parse errors) are logged as warnings and skipped -- a single broken plugin does not prevent others from loading.

### Why Two Directories?

User-level plugins are for capabilities you always want, regardless of project. A "journal" skill that helps you write daily notes, a "code review" agent that you use across all repositories.

Project-level plugins are for capabilities tied to a specific codebase. A "deploy" skill that knows your project's deployment process, a "test" agent configured with your project's testing conventions.

Scanning both directories means plugins compose without configuration. Drop a plugin directory in the right place and it is available.

## Plugin Components

### Skills

A skill is a prompt template that the agent can incorporate into its system prompt. Skills are defined as SKILL.md files with YAML frontmatter:

```markdown
---
name: code-review
description: Reviews code changes for quality and correctness
---

# Code Review Skill

When asked to review code, follow this process:

1. Read the changed files
2. Check for common issues: ...
3. Provide structured feedback: ...
```

The frontmatter provides metadata (name, description) while the markdown body provides the actual instructions. When the agent invokes this skill, the markdown content is injected into the system prompt, giving the agent specialized knowledge for the task.

Skills are lightweight by design. They do not contain executable code -- they are pure instructions. This makes them safe to distribute and easy to understand. You can read a SKILL.md file and know exactly what the agent will do differently when the skill is active.

### Plugin-Provided Agents

Plugins can define specialized agent configurations. An agent definition is a markdown file with frontmatter specifying the agent's description, allowed tools, and system prompt:

```markdown
---
description: Security analysis agent
tools: ["shell", "file_read", "grep", "web_search"]
---

# Security Analyzer

You are a security analysis agent. Your job is to:
1. Identify potential vulnerabilities in the codebase
2. Check for common security anti-patterns
3. Report findings with severity levels
```

Plugin-provided agents are registered in the agent registry and can be invoked via the `delegate` tool. When the primary agent encounters a task that matches a specialized agent's description, it can delegate to that agent, which runs with its own system prompt, tool set, and constraints.

This delegation model enables specialization without coupling. The primary agent does not need to know how to do security analysis -- it just needs to know that the security analyzer agent exists and can be delegated to.

### Hooks

Hooks are shell commands that fire at lifecycle events. They are the most powerful plugin component and the most security-sensitive.

## The Hook System

### HookDispatcher

The `HookDispatcher` is the runtime engine for hooks. It stores compiled hooks grouped by `HookEvent`:

- **PreToolUse**: Fires before a tool executes. Can block execution.
- **PostToolUse**: Fires after a tool executes. Informational only.
- **SessionStart**: Fires when a session begins.
- **SessionEnd**: Fires when a session closes.

Each hook has two optional matchers compiled at registration time:

- **tool_match**: A glob pattern matched against the tool name (e.g., `"Bash"`, `"file_*"`, `"*"`).
- **match_pattern**: A regex matched against the JSON-serialized tool parameters.

Both matchers must pass for the hook to fire. If neither matcher is specified, the hook fires for every event of its type.

### Hook Execution Flow

When a hook event occurs:

1. The dispatcher finds all hooks registered for that event type.
2. For each hook, it checks the tool_match glob and match_pattern regex.
3. Matching hooks are executed as shell subprocesses.
4. The subprocess receives a JSON context object on stdin.
5. The subprocess has a 10-second timeout (configurable).
6. For PreToolUse: a non-zero exit code blocks the tool call.
7. For PostToolUse: exit code is logged but does not block.

### PreToolUse: The Blocking Hook

PreToolUse hooks are the most powerful hook type because they can prevent tool execution. This enables use cases like:

- **Custom security policies**: Block specific commands based on project rules that go beyond the built-in CommandValidator.
- **Audit logging**: Log every shell command to a compliance audit trail.
- **Approval workflows**: Require human approval for certain operations (the hook could prompt for confirmation).
- **Cost controls**: Block web API calls when a budget is exceeded.

The blocking model is "first blocker wins." If any hook returns non-zero, execution is blocked immediately. The remaining hooks for that event are not checked. The blocking hook's stdout becomes the error message returned to the LLM.

### PostToolUse: The Observer Hook

PostToolUse hooks cannot block (the tool already executed). They receive the tool name, parameters, and result. Common uses include:

- **Metrics collection**: Track tool usage patterns.
- **Side effects**: Update external systems based on tool results.
- **Logging**: Record tool execution for later analysis.

### Why 10-Second Timeout?

Hook subprocesses have a default timeout of 10 seconds. This is long enough for a script to make an API call or query a database, but short enough to prevent a hung hook from blocking the agent indefinitely.

The timeout is configurable because some use cases legitimately need more time (e.g., a hook that runs a security scanner). But the default is conservative -- a hook should be fast because it runs on every matching tool call.

## Plugin Prompt Injection

Beyond hooks, plugins can inject content into the system prompt. The `SystemPromptBuilder` maintains a `plugin_prompts` vector. Each loaded plugin can contribute prompt text that is included in every turn.

This mechanism is how skills work at the prompt level: the skill's markdown content is added to `plugin_prompts`, and the system prompt builder includes it alongside the bootstrap prompt, tool documentation, and workspace context.

The order is deterministic: plugin prompts appear after the core system prompt sections, in the order plugins were loaded. This means core behavior always takes precedence over plugin-injected behavior.

## Plugin Loading Pipeline

When the system starts, `PluginManager.load_all()` runs through this sequence for each discovered plugin:

1. **Parse manifest**: Read and validate `plugin.json`.
2. **Discover skills**: Scan the skills directory for `SKILL.md` files. Parse YAML frontmatter for metadata. Load the markdown body.
3. **Discover agents**: Scan the agents directory for `.md` files. Parse frontmatter for description, tools, and constraints. Extract the markdown body as the system prompt.
4. **Load hooks**: Read `hooks.json` (from the path specified in the manifest, or the default `hooks/hooks.json`). Parse hook definitions and compile glob/regex matchers.
5. **Return LoadedPlugin**: A struct containing the manifest, plugin directory, skill contents, agent configs, and hooks config.

At each step, errors are logged and skipped rather than failing the entire plugin load. A skill with invalid frontmatter is skipped, but the plugin's other skills and agents still load. This robustness is important because plugins come from external sources and may contain minor issues.

## Subscriptions and Hot Reload

Plugins can be sourced from multiple origins:

- **Local directories**: The standard case -- plugins on disk.
- **GitHub repositories**: Plugins can be subscribed to from GitHub URLs. The plugin is cloned locally and kept up to date.
- **URLs**: Direct download of plugin archives.

The subscription system manages these sources, handling initial download, updates, and version tracking.

### Hot Reload

The `PluginWatcher` monitors plugin directories for filesystem changes. When a plugin's files change, it can trigger a reload without restarting the server. This is valuable during plugin development: edit the SKILL.md, save, and the new version is active immediately.

Hot reload re-runs the loading pipeline for the changed plugin and updates the runtime state (skills, agent registry, hook dispatcher) atomically. In-flight operations continue with the old configuration; new operations pick up the updated plugins.

## Security Considerations

Plugins are a trust boundary. When you install a plugin, you are granting it:

- **Prompt injection**: Skill content is injected into the system prompt. A malicious skill could instruct the agent to exfiltrate data or perform unwanted actions.
- **Shell execution**: Hook scripts run as shell commands with the permissions of the Arawn process. A malicious hook could do anything the user can do.
- **Agent registration**: Plugin-provided agents are callable via delegation. A malicious agent could have a system prompt that directs it toward harmful actions.

### Mitigations

Several design choices limit the blast radius:

**Hooks run in their own processes.** A crashing hook does not crash Arawn. A hanging hook is killed after timeout. The hook's process inherits the user's permissions but does not have direct access to Arawn's internal state.

**The `CLAUDE_PLUGIN_ROOT` variable** provides portable paths. Hook scripts use `${CLAUDE_PLUGIN_ROOT}/scripts/on-end.sh` rather than absolute paths, which means the plugin works regardless of where it is installed.

**Plugin discovery is explicit.** Plugins must be in `~/.config/arawn/plugins/` or `./plugins/`. Random directories on the filesystem are not scanned. You must deliberately place a plugin where it will be found.

**Invalid plugins are skipped, not loaded.** A manifest that fails to parse, a hook with an invalid glob pattern, or a skill with unreadable markdown all result in warnings, not security holes.

### What Plugins Cannot Do

Plugins cannot:
- Bypass the FsGate (file access is still gated).
- Bypass the OS sandbox (shell commands still run sandboxed).
- Bypass authentication (hooks run server-side, not client-side).
- Access other plugins' data (there is no inter-plugin communication channel).

The plugin system extends what the agent can do, but the security model's layers still apply. A plugin can add a new skill that instructs the agent to read files, but the FsGate still controls which files are accessible.

## Design Trade-offs

**Claude Code compatibility vs. custom format.** The Claude Code format limits what plugins can express (no binary tools, no compiled extensions). A custom format could support richer plugin types but would mean starting from zero. Compatibility was chosen to bootstrap the plugin ecosystem.

**Shell subprocesses for hooks vs. in-process plugins.** Shell subprocesses add latency (process spawn per hook call) but provide isolation and language independence. In-process plugins would be faster but could crash the host, require a specific runtime, and make the security model much harder to reason about.

**Skills as prompt text vs. executable code.** Skills are pure markdown instructions, not executable code. This limits their power (a skill cannot define a new tool) but makes them safe and portable. The security properties of a skill are exactly the security properties of its text content -- there are no hidden behaviors.
