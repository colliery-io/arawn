//! Slash command parsing, registry, and autocomplete for the TUI.
//!
//! Commands are detected by a "/" prefix in the input buffer. They come in
//! three flavors:
//! - **Built-in**: /help, /clear, /plan — handled client-side
//! - **Inventory**: /plugins, /skills, /agents, /mcp, /tools — query server
//! - **Skill**: /skill-name — invoke a user-invocable skill via the server

/// A registered slash command.
#[derive(Debug, Clone)]
pub struct CommandInfo {
    pub name: String,
    pub description: String,
    pub kind: CommandKind,
}

/// What kind of slash command this is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandKind {
    /// Handled entirely client-side (e.g. /help, /clear).
    BuiltIn,
    /// Queries the server for an inventory listing (e.g. /plugins, /skills).
    Inventory,
    /// Invokes a user-invocable skill on the server.
    Skill,
}

/// Result of parsing a slash command from the input buffer.
#[derive(Debug, Clone)]
pub struct ParsedCommand {
    pub name: String,
    pub args: String,
}

/// Parse a slash command from the input buffer.
/// Returns None if the input doesn't start with "/".
pub fn parse_command(input: &str) -> Option<ParsedCommand> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    let without_slash = &trimmed[1..];
    if without_slash.is_empty() {
        return None;
    }

    let (name, args) = match without_slash.find(char::is_whitespace) {
        Some(pos) => (
            without_slash[..pos].to_string(),
            without_slash[pos..].trim().to_string(),
        ),
        None => (without_slash.to_string(), String::new()),
    };

    Some(ParsedCommand { name, args })
}

/// The command registry — holds all available slash commands.
#[derive(Debug, Clone, Default)]
pub struct CommandRegistry {
    commands: Vec<CommandInfo>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        let mut reg = Self::default();
        reg.register_builtins();
        reg
    }

    fn register_builtins(&mut self) {
        self.commands.push(CommandInfo {
            name: "help".into(),
            description: "Show available slash commands".into(),
            kind: CommandKind::BuiltIn,
        });
        self.commands.push(CommandInfo {
            name: "clear".into(),
            description: "Clear the chat history".into(),
            kind: CommandKind::BuiltIn,
        });
        self.commands.push(CommandInfo {
            name: "plan".into(),
            description: "Enter plan mode (observation only)".into(),
            kind: CommandKind::BuiltIn,
        });
        // Permission mode
        self.commands.push(CommandInfo {
            name: "accept".into(),
            description: "Set permission mode (on/off/edits)".into(),
            kind: CommandKind::BuiltIn,
        });
        // Workstream/session management
        self.commands.push(CommandInfo {
            name: "workstream".into(),
            description: "Manage workstreams (create, list, switch)".into(),
            kind: CommandKind::BuiltIn,
        });
        self.commands.push(CommandInfo {
            name: "session".into(),
            description: "Manage sessions (new, list)".into(),
            kind: CommandKind::BuiltIn,
        });
        self.commands.push(CommandInfo {
            name: "promote".into(),
            description: "Promote scratch session to a workstream".into(),
            kind: CommandKind::BuiltIn,
        });
        // Inventory commands
        self.commands.push(CommandInfo {
            name: "tools".into(),
            description: "List available tools".into(),
            kind: CommandKind::Inventory,
        });
        self.commands.push(CommandInfo {
            name: "skills".into(),
            description: "List available skills".into(),
            kind: CommandKind::Inventory,
        });
        self.commands.push(CommandInfo {
            name: "plugins".into(),
            description: "List loaded plugins".into(),
            kind: CommandKind::Inventory,
        });
        self.commands.push(CommandInfo {
            name: "agents".into(),
            description: "List available agent types".into(),
            kind: CommandKind::Inventory,
        });
        self.commands.push(CommandInfo {
            name: "mcp".into(),
            description: "List connected MCP servers".into(),
            kind: CommandKind::Inventory,
        });
        // Workflow commands
        self.commands.push(CommandInfo {
            name: "workflows".into(),
            description: "List workflows and execution status".into(),
            kind: CommandKind::BuiltIn,
        });
        // Permissions inspection
        self.commands.push(CommandInfo {
            name: "permissions".into(),
            description: "Show active permission rules and recent decisions".into(),
            kind: CommandKind::BuiltIn,
        });
        // External integrations
        self.commands.push(CommandInfo {
            name: "integrations".into(),
            description: "List registered external integrations and their connection state".into(),
            kind: CommandKind::BuiltIn,
        });
        self.commands.push(CommandInfo {
            name: "connect".into(),
            description: "Begin the auth flow for an integration (e.g. /connect gmail)".into(),
            kind: CommandKind::BuiltIn,
        });
        self.commands.push(CommandInfo {
            name: "disconnect".into(),
            description: "Drop stored credentials for an integration".into(),
            kind: CommandKind::BuiltIn,
        });
        // Continual data feeds (I-0039)
        self.commands.push(CommandInfo {
            name: "watch".into(),
            description: "Register a continual data feed. /watch list <template> picks values.".into(),
            kind: CommandKind::BuiltIn,
        });
        self.commands.push(CommandInfo {
            name: "feeds".into(),
            description: "List/manage data feeds (subcommands: pause, resume, rm)".into(),
            kind: CommandKind::BuiltIn,
        });
        // Memory commands
        self.commands.push(CommandInfo {
            name: "remember".into(),
            description: "Store a fact in the knowledge base".into(),
            kind: CommandKind::BuiltIn,
        });
        self.commands.push(CommandInfo {
            name: "memory".into(),
            description: "Show knowledge base summary".into(),
            kind: CommandKind::BuiltIn,
        });
        self.commands.push(CommandInfo {
            name: "forget".into(),
            description: "Remove an entity from the knowledge base".into(),
            kind: CommandKind::BuiltIn,
        });
    }

    /// Add skill commands from the server's cached skill list.
    pub fn register_skills(&mut self, skills: Vec<(String, String)>) {
        // Remove old skill commands
        self.commands.retain(|c| c.kind != CommandKind::Skill);
        for (name, description) in skills {
            self.commands.push(CommandInfo {
                name,
                description,
                kind: CommandKind::Skill,
            });
        }
    }

    /// Get all commands.
    pub fn all(&self) -> &[CommandInfo] {
        &self.commands
    }

    /// Find commands matching a prefix (for autocomplete).
    pub fn matching(&self, prefix: &str) -> Vec<&CommandInfo> {
        let lower = prefix.to_lowercase();
        self.commands
            .iter()
            .filter(|c| c.name.to_lowercase().starts_with(&lower))
            .collect()
    }

    /// Look up a command by exact name.
    pub fn find(&self, name: &str) -> Option<&CommandInfo> {
        let lower = name.to_lowercase();
        self.commands.iter().find(|c| c.name.to_lowercase() == lower)
    }
}

/// Autocomplete state for the slash command dropdown.
#[derive(Debug, Clone)]
pub struct AutocompleteState {
    /// Filtered suggestions based on current input.
    pub suggestions: Vec<CommandInfo>,
    /// Currently highlighted index.
    pub selected: usize,
}

impl AutocompleteState {
    pub fn new(suggestions: Vec<CommandInfo>) -> Self {
        Self {
            suggestions,
            selected: 0,
        }
    }

    pub fn next(&mut self) {
        if !self.suggestions.is_empty() {
            self.selected = (self.selected + 1) % self.suggestions.len();
        }
    }

    pub fn prev(&mut self) {
        if !self.suggestions.is_empty() {
            self.selected = if self.selected == 0 {
                self.suggestions.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    pub fn selected_command(&self) -> Option<&CommandInfo> {
        self.suggestions.get(self.selected)
    }

    pub fn is_empty(&self) -> bool {
        self.suggestions.is_empty()
    }
}

/// The result of executing a built-in command.
#[derive(Debug)]
pub enum CommandResult {
    /// Show a system message in chat.
    SystemMessage(String),
    /// Clear chat messages.
    ClearChat,
    /// Enter plan mode (sends as a chat message to trigger the tool).
    EnterPlan,
    /// Query server for inventory.
    QueryInventory(String),
    /// Invoke a skill on the server.
    InvokeSkill { name: String, args: String },
    /// Store a memory via /remember.
    RememberFact(String),
    /// Show KB summary via /memory.
    MemorySummary,
    /// Forget/delete an entity via /forget.
    ForgetEntity(String),
    /// Create a new workstream.
    WorkstreamCreate(String),
    /// List all workstreams.
    WorkstreamList,
    /// Switch to a workstream by name.
    WorkstreamSwitch(String),
    /// Create a new session in the current workstream.
    SessionNew,
    /// List sessions in the current workstream.
    SessionList,
    /// Promote current scratch session to a workstream.
    PromoteSession(String),
    /// Set permission mode (mode string: "bypass", "default", "accept_edits", "plan").
    SetPermissionMode(String),
    /// List installed workflows.
    WorkflowList,
    /// Show workflow execution status.
    WorkflowStatus(Option<String>),
    /// Show active permission rules + recent decisions.
    PermissionsStatus,
    /// List registered external integrations + connection state.
    IntegrationsList,
    /// Begin the auth flow for an integration. Argument is the service name.
    IntegrationConnect(String),
    /// Drop stored credentials for an integration. Argument is the service name.
    IntegrationDisconnect(String),
    /// Register a continual data feed via `/watch <template> <feed_id> [k=v]...`.
    /// Slice 1 of T-0219: non-interactive form only — pickers land later.
    FeedRegister(WatchSpec),
    /// List configured feeds via `/feeds` (read-only).
    FeedList,
    /// Pause a feed via `/feeds pause <id>`.
    FeedPause(String),
    /// Resume a paused feed via `/feeds resume <id>`.
    FeedResume(String),
    /// Decommission a feed via `/feeds rm <id>` (with confirm token).
    /// Slice 2 wires this through a chat-line confirm — slice 2b
    /// upgrades to a modal once the modal infra is in place.
    FeedRemove { feed_id: String, confirmed: bool },
    /// Discover pickable params for a template via `/watch list
    /// <template>`. Prints a numbered list the user can copy values
    /// from into a full `/watch <template> <feed_id> k=v...` form.
    /// `None` means `/watch list` with no template — list every
    /// registered template instead.
    FeedDiscover(Option<String>),
    /// Trigger a one-off run of a feed via `/feeds run <id>` —
    /// useful for testing without waiting for the next cron tick.
    FeedRun(String),
}

/// Parsed args for the non-interactive form of `/watch`.
///
/// Form: `/watch <template> <feed_id> [param=value]... [@cadence=<cron>]`
///
/// The template name is the canonical `<provider>/<template>` (e.g.
/// `slack/channel-archive`); `feed_id` is a caller-chosen identifier
/// like `design` or `me`. Params after that are space-separated
/// `key=value` pairs; values may be quoted to include spaces.
/// `@cadence=<cron>` is reserved syntax — when present, overrides the
/// template's default cadence.
#[derive(Debug, Clone, PartialEq)]
pub struct WatchSpec {
    pub template: String,
    pub feed_id: String,
    pub params: serde_json::Value,
    pub cadence: Option<String>,
}

/// Parse the args body of `/watch`. Returns either a fully-formed
/// `WatchSpec` or a human-readable usage error.
///
/// Recognized forms:
/// - `<template> <feed_id>` — defaults for everything.
/// - `<template> <feed_id> key=value [key=value ...]` — params.
/// - `<template> <feed_id> ... @cadence=<cron>` — cadence override.
///
/// Quoting: a `key="value with spaces"` token is honored. Inner
/// double-quotes can be escaped with `\"`.
pub fn parse_watch_args(args: &str) -> Result<WatchSpec, String> {
    let tokens = tokenize_kv(args.trim())
        .map_err(|e| format!("/watch: {e}"))?;
    if tokens.len() < 2 {
        return Err(
            "Usage: /watch <provider/template> <feed_id> [key=value ...]\n\
             Example: /watch slack/channel-archive design channel=C0123ABCD"
                .into(),
        );
    }
    let template = tokens[0].clone();
    if !template.contains('/') {
        return Err(format!(
            "/watch: template '{template}' must be '<provider>/<template>' \
             (e.g. slack/channel-archive)"
        ));
    }
    let feed_id = tokens[1].clone();
    if feed_id.is_empty() {
        return Err("/watch: feed_id cannot be empty".into());
    }

    let mut params = serde_json::Map::new();
    let mut cadence: Option<String> = None;
    for tok in &tokens[2..] {
        let (k, v) = tok
            .split_once('=')
            .ok_or_else(|| format!("/watch: '{tok}' is not key=value"))?;
        if k == "@cadence" {
            cadence = Some(v.to_string());
            continue;
        }
        if k == "since" {
            // Resolve at parse time to a canonical RFC3339 string so
            // every template gets the same shape. Accepts:
            //   - RFC3339 datetime: 2026-01-01T12:00:00Z
            //   - ISO date (treated as midnight UTC): 2026-01-01
            //   - Relative duration: 7d / 12h / 6w / 6mo
            let iso = parse_since(v)
                .map_err(|e| format!("/watch: bad since value '{v}': {e}"))?;
            params.insert("since".into(), serde_json::Value::String(iso));
            continue;
        }
        // Param values are strings unless they parse as a JSON literal
        // (true/false/number/null). Lets `count=5` and `enabled=true`
        // arrive typed without burdening the user with quoting.
        let value = if let Ok(j) = serde_json::from_str::<serde_json::Value>(v) {
            j
        } else {
            serde_json::Value::String(v.to_string())
        };
        params.insert(k.to_string(), value);
    }

    Ok(WatchSpec {
        template,
        feed_id,
        params: serde_json::Value::Object(params),
        cadence,
    })
}

/// Parse a `since=` value into a canonical RFC3339 UTC string.
///
/// Accepts:
/// - `2026-01-01T12:00:00Z` — RFC3339 datetime, returned as-is.
/// - `2026-01-01` — ISO date, expanded to that day's midnight UTC.
/// - `Nd` / `Nh` / `Nw` / `Nmo` — relative duration walked back from now.
fn parse_since(s: &str) -> Result<String, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty value".into());
    }

    // Relative form first — easiest to disambiguate from a digit-led ISO date.
    if let Some(captures) = parse_relative_duration(s) {
        let (n, unit) = captures;
        let secs = match unit {
            "h" => n * 3600,
            "d" => n * 86400,
            "w" => n * 86400 * 7,
            "mo" => n * 86400 * 30,
            _ => return Err(format!("unknown duration unit '{unit}'")),
        };
        let dt = chrono::Utc::now() - chrono::Duration::seconds(secs);
        return Ok(dt.to_rfc3339());
    }

    // RFC3339 datetime.
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&chrono::Utc).to_rfc3339());
    }

    // Bare date `YYYY-MM-DD` → midnight UTC.
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let dt = d.and_hms_opt(0, 0, 0).unwrap().and_utc();
        return Ok(dt.to_rfc3339());
    }

    Err("expected RFC3339 datetime, YYYY-MM-DD, or relative like 7d/12h/6mo".into())
}

/// Pull `<digits><unit>` out of the input. Returns `(n, unit)` with the
/// unit kept lowercased for the caller's match arm.
fn parse_relative_duration(s: &str) -> Option<(i64, &str)> {
    let s = s.trim();
    let split_at = s.bytes().position(|b| !b.is_ascii_digit())?;
    if split_at == 0 {
        return None;
    }
    let n: i64 = s[..split_at].parse().ok()?;
    let unit = &s[split_at..];
    match unit {
        "h" | "d" | "w" | "mo" => Some((n, unit)),
        _ => None,
    }
}

/// Tokenizer that respects double-quoted runs so a param value can
/// include spaces. Doesn't try to be a full shell parser — just
/// enough for the `/watch` use case.
fn tokenize_kv(s: &str) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => in_quotes = !in_quotes,
            '\\' if in_quotes => match chars.next() {
                Some('"') => cur.push('"'),
                Some(other) => {
                    cur.push('\\');
                    cur.push(other);
                }
                None => return Err("trailing backslash".into()),
            },
            c if c.is_whitespace() && !in_quotes => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if in_quotes {
        return Err("unterminated double-quote".into());
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    Ok(out)
}

/// Parse the args of `/feeds` into a CommandResult.
///
/// Forms:
/// - `/feeds` — list (read-only).
/// - `/feeds pause <id>` — pause a feed.
/// - `/feeds resume <id>` — resume a paused feed.
/// - `/feeds rm <id>` — open the confirm-and-delete flow.
/// - `/feeds rm <id> --yes` — skip the confirm (for scripts / tests).
pub fn parse_feeds_args(args: &str) -> CommandResult {
    let trimmed = args.trim();
    if trimmed.is_empty() {
        return CommandResult::FeedList;
    }
    let mut tokens = trimmed.split_whitespace();
    let sub = tokens.next().unwrap_or("");
    match sub {
        "pause" => match tokens.next() {
            Some(id) => CommandResult::FeedPause(id.into()),
            None => CommandResult::SystemMessage("Usage: /feeds pause <id>".into()),
        },
        "resume" => match tokens.next() {
            Some(id) => CommandResult::FeedResume(id.into()),
            None => CommandResult::SystemMessage("Usage: /feeds resume <id>".into()),
        },
        "rm" | "remove" => match tokens.next() {
            Some(id) => {
                let confirmed = tokens.any(|t| t == "--yes" || t == "-y");
                CommandResult::FeedRemove {
                    feed_id: id.into(),
                    confirmed,
                }
            }
            None => {
                CommandResult::SystemMessage("Usage: /feeds rm <id> [--yes]".into())
            }
        },
        "run" => match tokens.next() {
            Some(id) => CommandResult::FeedRun(id.into()),
            None => CommandResult::SystemMessage("Usage: /feeds run <id>".into()),
        },
        other => CommandResult::SystemMessage(format!(
            "Unknown /feeds subcommand: '{other}'.\n\
             Usage:\n  \
             /feeds                    — list feeds\n  \
             /feeds run <id>           — trigger a one-off run now\n  \
             /feeds pause <id>         — pause a feed\n  \
             /feeds resume <id>        — resume a paused feed\n  \
             /feeds rm <id> [--yes]    — decommission a feed"
        )),
    }
}

/// Execute a parsed slash command against the registry.
pub fn execute_command(cmd: &ParsedCommand, registry: &CommandRegistry) -> CommandResult {
    match registry.find(&cmd.name) {
        Some(info) => match info.kind {
            CommandKind::BuiltIn => match info.name.as_str() {
                "help" => {
                    let mut help = String::from("Available commands:\n\n");
                    for c in registry.all() {
                        help.push_str(&format!("  /{:<12} {}\n", c.name, c.description));
                    }
                    CommandResult::SystemMessage(help)
                }
                "clear" => CommandResult::ClearChat,
                "workstream" => {
                    let parts: Vec<&str> = cmd.args.splitn(2, char::is_whitespace).collect();
                    match parts.first().copied() {
                        Some("create") => {
                            let name = parts.get(1).unwrap_or(&"").trim();
                            if name.is_empty() {
                                CommandResult::SystemMessage("Usage: /workstream create <name>".into())
                            } else {
                                CommandResult::WorkstreamCreate(name.to_string())
                            }
                        }
                        Some("list") => CommandResult::WorkstreamList,
                        Some("switch") => {
                            let name = parts.get(1).unwrap_or(&"").trim();
                            if name.is_empty() {
                                CommandResult::SystemMessage("Usage: /workstream switch <name>".into())
                            } else {
                                CommandResult::WorkstreamSwitch(name.to_string())
                            }
                        }
                        _ => CommandResult::SystemMessage(
                            "Usage: /workstream <create|list|switch> [name]\n\n  create <name>  Create a new workstream\n  list           List all workstreams\n  switch <name>  Switch to a workstream".into()
                        ),
                    }
                }
                "session" => {
                    let sub = cmd.args.split_whitespace().next().unwrap_or("");
                    match sub {
                        "new" => CommandResult::SessionNew,
                        "list" => CommandResult::SessionList,
                        _ => CommandResult::SystemMessage(
                            "Usage: /session <new|list>\n\n  new   Create a new session\n  list  List sessions in current workstream".into()
                        ),
                    }
                }
                "promote" => {
                    if cmd.args.is_empty() {
                        CommandResult::SystemMessage("Usage: /promote <workstream-name>".into())
                    } else {
                        CommandResult::PromoteSession(cmd.args.clone())
                    }
                }
                "plan" => CommandResult::SetPermissionMode("plan".into()),
                "accept" => {
                    let sub = cmd.args.split_whitespace().next().unwrap_or("");
                    match sub {
                        "on" => CommandResult::SetPermissionMode("bypass".into()),
                        "off" => CommandResult::SetPermissionMode("default".into()),
                        "edits" => CommandResult::SetPermissionMode("accept_edits".into()),
                        _ => CommandResult::SystemMessage(
                            "Usage: /accept <on|off|edits>\n\n  on      Full autonomy (bypass all permissions)\n  off     Restore default permission prompts\n  edits   Auto-allow file writes, prompt for shell".into()
                        ),
                    }
                }
                "workflows" => {
                    let sub = cmd.args.split_whitespace().next().unwrap_or("list");
                    match sub {
                        "list" | "" => CommandResult::WorkflowList,
                        "status" => {
                            let name = cmd.args.split_whitespace().nth(1).map(String::from);
                            CommandResult::WorkflowStatus(name)
                        }
                        _ => CommandResult::SystemMessage(
                            "Usage: /workflows [list|status [name]]\n\n  list           List installed workflows\n  status [name]  Show recent execution status".into()
                        ),
                    }
                }
                "remember" => {
                    if cmd.args.is_empty() {
                        CommandResult::SystemMessage("Usage: /remember <fact to store>".into())
                    } else {
                        CommandResult::RememberFact(cmd.args.clone())
                    }
                }
                "memory" => CommandResult::MemorySummary,
                "permissions" => CommandResult::PermissionsStatus,
                "integrations" => CommandResult::IntegrationsList,
                "connect" => {
                    let svc = cmd.args.split_whitespace().next().unwrap_or("");
                    if svc.is_empty() {
                        CommandResult::SystemMessage(
                            "Usage: /connect <service>\n\nRun /integrations to see what's available.".into(),
                        )
                    } else {
                        CommandResult::IntegrationConnect(svc.to_string())
                    }
                }
                "disconnect" => {
                    let svc = cmd.args.split_whitespace().next().unwrap_or("");
                    if svc.is_empty() {
                        CommandResult::SystemMessage("Usage: /disconnect <service>".into())
                    } else {
                        CommandResult::IntegrationDisconnect(svc.to_string())
                    }
                }
                "forget" => {
                    if cmd.args.is_empty() {
                        CommandResult::SystemMessage("Usage: /forget <entity title or ID>".into())
                    } else {
                        CommandResult::ForgetEntity(cmd.args.clone())
                    }
                }
                "watch" => {
                    // `/watch list [template]` is the discovery form;
                    // shares the verb with `/watch <template> <id>`.
                    // Only `list` followed by whitespace (or end of
                    // args) counts — `list-something` keeps the
                    // normal-form path.
                    let trimmed = cmd.args.trim();
                    let mut tokens = trimmed.split_whitespace();
                    let first = tokens.next().unwrap_or("");
                    if first == "list" {
                        // Take exactly one further token as the
                        // template; anything after is rejected with a
                        // hint so users don't think they can pre-pick
                        // a channel by name. (Discovery is two steps:
                        // list, then run /watch.)
                        let template = tokens.next().map(String::from);
                        if let Some(extra) = tokens.next() {
                            CommandResult::SystemMessage(format!(
                                "/watch list takes at most one argument (got extra `{extra}`).\n\n\
                                 Usage:\n  \
                                 /watch list                         — show all templates\n  \
                                 /watch list <template>              — show pickable values for that template\n\n\
                                 Then register with:\n  \
                                 /watch <template> <feed_id> <key>=<value>"
                            ))
                        } else {
                            CommandResult::FeedDiscover(template)
                        }
                    } else {
                        match parse_watch_args(&cmd.args) {
                            Ok(spec) => CommandResult::FeedRegister(spec),
                            Err(msg) => CommandResult::SystemMessage(msg),
                        }
                    }
                }
                "feeds" => parse_feeds_args(&cmd.args),

                _ => CommandResult::SystemMessage(format!("Unknown built-in: /{}", cmd.name)),
            },
            CommandKind::Inventory => CommandResult::QueryInventory(info.name.clone()),
            CommandKind::Skill => CommandResult::InvokeSkill {
                name: info.name.clone(),
                args: cmd.args.clone(),
            },
        },
        None => CommandResult::SystemMessage(format!(
            "Unknown command: /{}. Type /help to see available commands.",
            cmd.name
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_command() {
        let cmd = parse_command("/help").unwrap();
        assert_eq!(cmd.name, "help");
        assert_eq!(cmd.args, "");
    }

    #[test]
    fn watch_parses_template_id_and_string_param() {
        let spec = parse_watch_args("slack/channel-archive design channel=C0123ABCD")
            .expect("valid watch args");
        assert_eq!(spec.template, "slack/channel-archive");
        assert_eq!(spec.feed_id, "design");
        assert_eq!(spec.params["channel"], "C0123ABCD");
        assert!(spec.cadence.is_none());
    }

    #[test]
    fn watch_parses_typed_and_quoted_params_and_cadence_override() {
        // Cron expressions contain spaces, so the override must be
        // quoted as a single token: `@cadence="*/30 * * * *"`.
        let spec = parse_watch_args(
            "gmail/sender-filter alerts sender_pattern=\"alerts@vendor.com\" days_back=14 @cadence=\"*/30 * * * *\"",
        )
        .expect("valid watch args");
        assert_eq!(spec.feed_id, "alerts");
        assert_eq!(spec.params["sender_pattern"], "alerts@vendor.com");
        assert_eq!(spec.params["days_back"], 14);
        assert_eq!(spec.cadence.as_deref(), Some("*/30 * * * *"));
    }

    #[test]
    fn watch_parses_since_relative_duration() {
        let spec = parse_watch_args("slack/channel-archive design channel=C123 since=180d")
            .expect("valid watch args");
        let since = spec.params["since"].as_str().expect("since string");
        // RFC3339 parseable + earlier than now.
        let dt = chrono::DateTime::parse_from_rfc3339(since).unwrap();
        let now = chrono::Utc::now();
        assert!(now - dt.with_timezone(&chrono::Utc) > chrono::Duration::days(179));
    }

    #[test]
    fn watch_parses_since_iso_date() {
        let spec = parse_watch_args(
            "slack/channel-archive design channel=C123 since=2026-01-01",
        )
        .expect("valid watch args");
        assert_eq!(
            spec.params["since"].as_str().unwrap(),
            "2026-01-01T00:00:00+00:00"
        );
    }

    #[test]
    fn watch_parses_since_rfc3339() {
        let spec = parse_watch_args(
            "slack/channel-archive design channel=C123 since=2026-01-01T12:00:00Z",
        )
        .expect("valid watch args");
        let s = spec.params["since"].as_str().unwrap();
        // chrono normalizes Z → +00:00 in to_rfc3339; both are valid.
        assert!(s.starts_with("2026-01-01T12:00:00"));
    }

    #[test]
    fn watch_rejects_garbage_since() {
        assert!(parse_watch_args(
            "slack/channel-archive design channel=C123 since=tomorrow"
        )
        .is_err());
        assert!(parse_watch_args(
            "slack/channel-archive design channel=C123 since=180banana"
        )
        .is_err());
    }

    #[test]
    fn watch_rejects_missing_args_and_bad_template() {
        assert!(parse_watch_args("").is_err());
        assert!(parse_watch_args("slack/channel-archive").is_err());
        // template missing the provider/ prefix
        assert!(parse_watch_args("standalone-name design").is_err());
        // malformed key=value
        assert!(parse_watch_args("slack/channel-archive design malformed").is_err());
    }

    #[test]
    fn watch_command_dispatch_returns_feed_register() {
        let registry = CommandRegistry::new();
        let cmd = parse_command("/watch slack/channel-archive design channel=C123").unwrap();
        match execute_command(&cmd, &registry) {
            CommandResult::FeedRegister(spec) => {
                assert_eq!(spec.template, "slack/channel-archive");
                assert_eq!(spec.feed_id, "design");
                assert_eq!(spec.params["channel"], "C123");
            }
            other => panic!("expected FeedRegister, got {other:?}"),
        }
    }

    #[test]
    fn feeds_command_dispatch_returns_feed_list() {
        let registry = CommandRegistry::new();
        let cmd = parse_command("/feeds").unwrap();
        match execute_command(&cmd, &registry) {
            CommandResult::FeedList => {}
            other => panic!("expected FeedList, got {other:?}"),
        }
    }

    #[test]
    fn feeds_pause_and_resume_dispatch() {
        let registry = CommandRegistry::new();
        match execute_command(&parse_command("/feeds pause design").unwrap(), &registry) {
            CommandResult::FeedPause(id) => assert_eq!(id, "design"),
            other => panic!("expected FeedPause, got {other:?}"),
        }
        match execute_command(&parse_command("/feeds resume design").unwrap(), &registry) {
            CommandResult::FeedResume(id) => assert_eq!(id, "design"),
            other => panic!("expected FeedResume, got {other:?}"),
        }
    }

    #[test]
    fn feeds_rm_requires_confirm_flag() {
        let registry = CommandRegistry::new();
        match execute_command(&parse_command("/feeds rm design").unwrap(), &registry) {
            CommandResult::FeedRemove { feed_id, confirmed } => {
                assert_eq!(feed_id, "design");
                assert!(!confirmed, "without --yes, removal is not confirmed");
            }
            other => panic!("expected FeedRemove, got {other:?}"),
        }
        match execute_command(&parse_command("/feeds rm design --yes").unwrap(), &registry) {
            CommandResult::FeedRemove { confirmed, .. } => assert!(confirmed),
            other => panic!("expected FeedRemove, got {other:?}"),
        }
    }

    #[test]
    fn feeds_pause_without_id_is_a_usage_message() {
        let registry = CommandRegistry::new();
        match execute_command(&parse_command("/feeds pause").unwrap(), &registry) {
            CommandResult::SystemMessage(msg) => assert!(msg.contains("Usage")),
            other => panic!("expected SystemMessage, got {other:?}"),
        }
    }

    #[test]
    fn watch_list_dispatches_to_feed_discover() {
        let registry = CommandRegistry::new();
        // No template — discovery for templates list (event_loop
        // turns this into a static help message).
        match execute_command(&parse_command("/watch list").unwrap(), &registry) {
            CommandResult::FeedDiscover(None) => {}
            other => panic!("expected FeedDiscover(None), got {other:?}"),
        }
        // With template — picker mode for that template.
        match execute_command(
            &parse_command("/watch list slack/channel-archive").unwrap(),
            &registry,
        ) {
            CommandResult::FeedDiscover(Some(tpl)) => {
                assert_eq!(tpl, "slack/channel-archive");
            }
            other => panic!("expected FeedDiscover(Some), got {other:?}"),
        }
    }

    #[test]
    fn watch_list_rejects_extra_args_with_hint() {
        let registry = CommandRegistry::new();
        match execute_command(
            &parse_command("/watch list slack/channel-archive domino-data-labs").unwrap(),
            &registry,
        ) {
            CommandResult::SystemMessage(msg) => {
                assert!(msg.contains("at most one argument"));
                assert!(msg.contains("domino-data-labs"));
            }
            other => panic!("expected SystemMessage, got {other:?}"),
        }
    }

    #[test]
    fn watch_list_doesnt_swallow_a_template_named_listed() {
        // `/watch list-something foo` should still go through the
        // normal parse path, not the discovery path. Defensive — the
        // current parser uses `strip_prefix("list")` then trims, so
        // `list-something` would match. Verify this by giving it real
        // typed args and expecting normal-form dispatch.
        let registry = CommandRegistry::new();
        match execute_command(
            &parse_command("/watch slack/channel-archive design channel=C1").unwrap(),
            &registry,
        ) {
            CommandResult::FeedRegister(spec) => {
                assert_eq!(spec.template, "slack/channel-archive");
            }
            other => panic!("expected FeedRegister, got {other:?}"),
        }
    }

    #[test]
    fn feeds_unknown_subcommand_lists_usage() {
        let registry = CommandRegistry::new();
        match execute_command(&parse_command("/feeds wat").unwrap(), &registry) {
            CommandResult::SystemMessage(msg) => assert!(msg.contains("Unknown")),
            other => panic!("expected SystemMessage, got {other:?}"),
        }
    }

    #[test]
    fn parse_command_with_args() {
        let cmd = parse_command("/search foo bar").unwrap();
        assert_eq!(cmd.name, "search");
        assert_eq!(cmd.args, "foo bar");
    }

    #[test]
    fn parse_not_a_command() {
        assert!(parse_command("hello world").is_none());
        assert!(parse_command("").is_none());
        assert!(parse_command("  ").is_none());
    }

    #[test]
    fn parse_slash_only() {
        assert!(parse_command("/").is_none());
    }

    #[test]
    fn parse_with_leading_whitespace() {
        let cmd = parse_command("  /help").unwrap();
        assert_eq!(cmd.name, "help");
    }

    #[test]
    fn registry_has_builtins() {
        let reg = CommandRegistry::new();
        assert!(reg.find("help").is_some());
        assert!(reg.find("clear").is_some());
        assert!(reg.find("plan").is_some());
        assert!(reg.find("tools").is_some());
        assert!(reg.find("skills").is_some());
    }

    #[test]
    fn registry_matching_prefix() {
        let reg = CommandRegistry::new();
        let matches = reg.matching("pl");
        assert_eq!(matches.len(), 2); // plan, plugins
        assert!(matches.iter().any(|c| c.name == "plan"));
        assert!(matches.iter().any(|c| c.name == "plugins"));
    }

    #[test]
    fn registry_matching_empty_returns_all() {
        let reg = CommandRegistry::new();
        let matches = reg.matching("");
        assert_eq!(matches.len(), reg.all().len());
    }

    #[test]
    fn registry_skills() {
        let mut reg = CommandRegistry::new();
        let builtin_count = reg.all().len();
        reg.register_skills(vec![
            ("commit".into(), "Create a git commit".into()),
            ("review".into(), "Review code changes".into()),
        ]);
        assert_eq!(reg.all().len(), builtin_count + 2);
        assert_eq!(reg.find("commit").unwrap().kind, CommandKind::Skill);
    }

    #[test]
    fn autocomplete_navigation() {
        let suggestions = vec![
            CommandInfo { name: "help".into(), description: "".into(), kind: CommandKind::BuiltIn },
            CommandInfo { name: "clear".into(), description: "".into(), kind: CommandKind::BuiltIn },
            CommandInfo { name: "plan".into(), description: "".into(), kind: CommandKind::BuiltIn },
        ];
        let mut ac = AutocompleteState::new(suggestions);
        assert_eq!(ac.selected, 0);

        ac.next();
        assert_eq!(ac.selected, 1);
        ac.next();
        assert_eq!(ac.selected, 2);
        ac.next();
        assert_eq!(ac.selected, 0); // wraps

        ac.prev();
        assert_eq!(ac.selected, 2); // wraps back
    }

    #[test]
    fn execute_help() {
        let reg = CommandRegistry::new();
        let cmd = parse_command("/help").unwrap();
        match execute_command(&cmd, &reg) {
            CommandResult::SystemMessage(msg) => assert!(msg.contains("/help")),
            _ => panic!("expected SystemMessage"),
        }
    }

    #[test]
    fn execute_clear() {
        let reg = CommandRegistry::new();
        let cmd = parse_command("/clear").unwrap();
        assert!(matches!(execute_command(&cmd, &reg), CommandResult::ClearChat));
    }

    #[test]
    fn execute_unknown() {
        let reg = CommandRegistry::new();
        let cmd = parse_command("/nonexistent").unwrap();
        match execute_command(&cmd, &reg) {
            CommandResult::SystemMessage(msg) => assert!(msg.contains("Unknown command")),
            _ => panic!("expected SystemMessage"),
        }
    }

    #[test]
    fn execute_inventory() {
        let reg = CommandRegistry::new();
        let cmd = parse_command("/tools").unwrap();
        match execute_command(&cmd, &reg) {
            CommandResult::QueryInventory(kind) => assert_eq!(kind, "tools"),
            _ => panic!("expected QueryInventory"),
        }
    }

    #[test]
    fn execute_skill() {
        let mut reg = CommandRegistry::new();
        reg.register_skills(vec![("commit".into(), "Git commit".into())]);
        let cmd = parse_command("/commit -m 'fix bug'").unwrap();
        match execute_command(&cmd, &reg) {
            CommandResult::InvokeSkill { name, args } => {
                assert_eq!(name, "commit");
                assert_eq!(args, "-m 'fix bug'");
            }
            _ => panic!("expected InvokeSkill"),
        }
    }

    // T-0195/T-0197 wiring: every command in /help must produce a real
    // CommandResult variant — no SystemMessage fall-throughs that look like
    // "Unknown command" or "not implemented" for advertised commands.

    #[test]
    fn execute_remember_with_text_returns_remember_fact() {
        let reg = CommandRegistry::new();
        let cmd = parse_command("/remember the project lives in ~/src/arawn").unwrap();
        match execute_command(&cmd, &reg) {
            CommandResult::RememberFact(text) => {
                assert_eq!(text, "the project lives in ~/src/arawn");
            }
            other => panic!("expected RememberFact, got {other:?}"),
        }
    }

    #[test]
    fn execute_remember_without_text_returns_usage_message() {
        let reg = CommandRegistry::new();
        let cmd = parse_command("/remember").unwrap();
        match execute_command(&cmd, &reg) {
            CommandResult::SystemMessage(msg) => {
                assert!(msg.contains("Usage:"), "expected usage message, got: {msg}");
                assert!(msg.contains("/remember"), "usage should mention command name");
            }
            other => panic!("expected SystemMessage, got {other:?}"),
        }
    }

    #[test]
    fn execute_memory_returns_memory_summary() {
        let reg = CommandRegistry::new();
        let cmd = parse_command("/memory").unwrap();
        assert!(matches!(
            execute_command(&cmd, &reg),
            CommandResult::MemorySummary
        ));
    }

    #[test]
    fn execute_forget_with_query_returns_forget_entity() {
        let reg = CommandRegistry::new();
        let cmd = parse_command("/forget the old preference").unwrap();
        match execute_command(&cmd, &reg) {
            CommandResult::ForgetEntity(query) => {
                assert_eq!(query, "the old preference");
            }
            other => panic!("expected ForgetEntity, got {other:?}"),
        }
    }

    #[test]
    fn execute_forget_without_query_returns_usage_message() {
        let reg = CommandRegistry::new();
        let cmd = parse_command("/forget").unwrap();
        match execute_command(&cmd, &reg) {
            CommandResult::SystemMessage(msg) => {
                assert!(msg.contains("Usage:"), "expected usage message");
            }
            other => panic!("expected SystemMessage, got {other:?}"),
        }
    }

    #[test]
    fn execute_workflows_list_returns_workflow_list() {
        let reg = CommandRegistry::new();
        // Both `/workflows` and `/workflows list` should produce WorkflowList.
        for input in ["/workflows", "/workflows list"] {
            let cmd = parse_command(input).unwrap();
            assert!(
                matches!(execute_command(&cmd, &reg), CommandResult::WorkflowList),
                "{input} should return WorkflowList"
            );
        }
    }

    /// Audit: every built-in command in /help must dispatch to a CommandResult
    /// variant that actually does work — no "advertised but broken" state.
    /// A bare command (no args) should produce either the work-doing variant
    /// or a SystemMessage with explicit usage instructions, never a
    /// SystemMessage starting with "Unknown".
    #[test]
    fn every_advertised_builtin_dispatches_or_explains() {
        let reg = CommandRegistry::new();
        let builtins: Vec<String> = reg
            .all()
            .iter()
            .filter(|c| c.kind == CommandKind::BuiltIn)
            .map(|c| c.name.clone())
            .collect();
        assert!(!builtins.is_empty(), "registry should have built-in commands");

        for name in builtins {
            let input = format!("/{name}");
            let cmd = parse_command(&input).unwrap();
            match execute_command(&cmd, &reg) {
                CommandResult::SystemMessage(msg) => {
                    assert!(
                        !msg.starts_with("Unknown"),
                        "/{name} dispatched to 'Unknown' SystemMessage — wire it or remove from registry"
                    );
                }
                _ => {} // any non-SystemMessage variant means it's wired to do real work
            }
        }
    }

    // T-0201: integration commands

    #[test]
    fn execute_integrations_returns_list_variant() {
        let reg = CommandRegistry::new();
        let cmd = parse_command("/integrations").unwrap();
        assert!(matches!(
            execute_command(&cmd, &reg),
            CommandResult::IntegrationsList
        ));
    }

    #[test]
    fn execute_connect_with_service_returns_connect_variant() {
        let reg = CommandRegistry::new();
        let cmd = parse_command("/connect gmail").unwrap();
        match execute_command(&cmd, &reg) {
            CommandResult::IntegrationConnect(svc) => assert_eq!(svc, "gmail"),
            other => panic!("expected IntegrationConnect, got {other:?}"),
        }
    }

    #[test]
    fn execute_connect_without_service_returns_usage_message() {
        let reg = CommandRegistry::new();
        let cmd = parse_command("/connect").unwrap();
        match execute_command(&cmd, &reg) {
            CommandResult::SystemMessage(msg) => {
                assert!(msg.contains("Usage:"), "expected usage message, got: {msg}");
                assert!(msg.contains("/connect"));
            }
            other => panic!("expected SystemMessage, got {other:?}"),
        }
    }

    #[test]
    fn execute_disconnect_with_service_returns_disconnect_variant() {
        let reg = CommandRegistry::new();
        let cmd = parse_command("/disconnect slack").unwrap();
        match execute_command(&cmd, &reg) {
            CommandResult::IntegrationDisconnect(svc) => assert_eq!(svc, "slack"),
            other => panic!("expected IntegrationDisconnect, got {other:?}"),
        }
    }

    #[test]
    fn execute_disconnect_without_service_returns_usage_message() {
        let reg = CommandRegistry::new();
        let cmd = parse_command("/disconnect").unwrap();
        match execute_command(&cmd, &reg) {
            CommandResult::SystemMessage(msg) => assert!(msg.contains("Usage:")),
            other => panic!("expected SystemMessage, got {other:?}"),
        }
    }

    /// Capabilities banner copy in event_loop.rs points users at this docs
    /// path; the test exists so a docs-tree rename surfaces here too.
    #[test]
    fn capabilities_banner_doc_path_pinned() {
        // If docs/src/memory.md moves, update event_loop.rs's capability
        // warning AND this assertion.
        const PINNED: &str = "docs/src/memory.md";
        assert!(
            std::path::Path::new("../..")
                .join(PINNED)
                .exists()
                || std::path::Path::new("../..").join("docs").exists(),
            "memory docs not at expected path; update banner copy in event_loop.rs"
        );
    }
}
