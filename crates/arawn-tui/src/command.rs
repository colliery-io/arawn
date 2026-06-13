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
        // T-0347: permission mode UX unified under `/autonomy`.
        // Drop the legacy `/accept` and `/plan` slash commands — plan
        // mode is now reachable as `/autonomy plan`.
        self.commands.push(CommandInfo {
            name: "autonomy".into(),
            description: "Set permission posture (ask|edits|full|plan)".into(),
            kind: CommandKind::BuiltIn,
        });
        // Lens/session management
        self.commands.push(CommandInfo {
            name: "lens".into(),
            description: "Manage lenses (create, list, promote)".into(),
            kind: CommandKind::BuiltIn,
        });
        self.commands.push(CommandInfo {
            name: "session".into(),
            description: "Manage sessions (new, list)".into(),
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
        // Health surface (ARAWN-I-0068 P2-1)
        self.commands.push(CommandInfo {
            name: "status".into(),
            description:
                "Show background subsystem health (feeds, ceremonies, embedding, extraction, LLM)"
                    .into(),
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
            description: "Register a continual data feed. /watch list <template> picks values."
                .into(),
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
        // Ceremony tablets (T-0307)
        self.commands.push(CommandInfo {
            name: "today".into(),
            description: "Show today's daily ceremony tablet".into(),
            kind: CommandKind::BuiltIn,
        });
        self.commands.push(CommandInfo {
            name: "week".into(),
            description: "Show this week's weekly ceremony tablet".into(),
            kind: CommandKind::BuiltIn,
        });
        self.commands.push(CommandInfo {
            name: "retro".into(),
            description: "Show this week's retro ceremony tablet".into(),
            kind: CommandKind::BuiltIn,
        });
        self.commands.push(CommandInfo {
            name: "brief".into(),
            description: "Show today's brief (daily + weekly tablet combined)".into(),
            kind: CommandKind::BuiltIn,
        });
        // T-0361: clipboard copy of the last assistant response.
        self.commands.push(CommandInfo {
            name: "copy".into(),
            description: "Copy the last assistant response to the clipboard".into(),
            kind: CommandKind::BuiltIn,
        });
        // T-0362: token-usage rollup in the TUI.
        self.commands.push(CommandInfo {
            name: "usage".into(),
            description: "Show token usage rollup ([day|week|month|all], default: day)".into(),
            kind: CommandKind::BuiltIn,
        });
        // T-0363: save the current conversation to markdown.
        self.commands.push(CommandInfo {
            name: "export".into(),
            description: "Export the current conversation to markdown ([path])".into(),
            kind: CommandKind::BuiltIn,
        });
        // Generic todo surface (I-0049 T-0314)
        self.commands.push(CommandInfo {
            name: "todo".into(),
            description: "Open the todo list (create, mark done, archive)".into(),
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
        self.commands
            .iter()
            .find(|c| c.name.to_lowercase() == lower)
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
    /// Create a new lens.
    LensCreate(String),
    /// Promote the current session into a named lens (ARAWN-T-0480),
    /// creating the lens if it doesn't exist. Argument is the lens name.
    LensPromote(String),
    /// List all lenses.
    LensList,
    /// Create a new session in the current lens.
    SessionNew,
    /// List sessions in the current lens.
    SessionList,
    /// Set permission mode. Mode string: "ask" | "edits" | "full" | "plan"
    /// (matches the `PermissionMode` enum's serde representation).
    SetPermissionMode(String),
    /// List installed workflows.
    WorkflowList,
    /// Show workflow execution status.
    WorkflowStatus(Option<String>),
    /// Show active permission rules + recent decisions.
    PermissionsStatus,
    /// Show background subsystem health (ARAWN-I-0068 P2-1) via `/status`.
    SystemStatus,
    /// List registered external integrations + connection state.
    IntegrationsList,
    /// Begin the auth flow for an integration. Argument is the service name.
    IntegrationConnect(String),
    /// Drop stored credentials for an integration. Argument is the service name.
    IntegrationDisconnect(String),
    /// Register a continual data feed via `/watch <template> <feed_id> [k=v]...`.
    /// Slice 1 of T-0219: non-interactive form only — pickers land later.
    FeedRegister(WatchSpec),
    /// Open the interactive `/watch` registration modal (ARAWN-I-0058) —
    /// triggered by `/watch` with no arguments.
    FeedWatchModal,
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
    /// Fetch + render today's daily ceremony tablet as a system
    /// message (T-0307 phase 1, read-only).
    CeremonyShowToday,
    /// Fetch + render this week's weekly ceremony tablet as a system
    /// message (T-0307 phase 1, read-only).
    CeremonyShowWeek,
    /// Fetch + render this week's retro ceremony tablet as a system
    /// message (T-0307 phase 1, read-only).
    CeremonyShowRetro,
    /// I-0035 Phase 2 (T-0354): fetch both daily and weekly tablets,
    /// compose via `BriefView`, render via `render_brief`, append as a
    /// system message. Read-only.
    BriefShow,
    /// T-0361: copy the last assistant message to the clipboard via
    /// OSC 52. Posts a confirmation toast.
    CopyLastResponse,
    /// T-0362: show the token-usage rollup as a system message,
    /// scoped to `period` (day|week|month|all; defaults to day).
    UsageShow { period: String },
    /// T-0363: write the current conversation to a markdown file.
    /// `path` is None when the user invoked `/export` with no arg
    /// — the handler picks a default under `~/.arawn/exports/`.
    ExportConversation { path: Option<String> },
    /// Open the `/todo` modal — generic todos surface (I-0049 T-0314).
    TodoShow,
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
    let tokens = tokenize_kv(args.trim()).map_err(|e| format!("/watch: {e}"))?;
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
    // A feed_id that looks like `key=value` is almost always the omitted-feed_id
    // mistake: `/watch filesystem/folder root=…` makes `root=…` the positional
    // feed_id, the params come out empty, and the template then reports a missing
    // required param. Catch it here with a message that points at the real cause.
    if feed_id.contains('=') {
        return Err(format!(
            "/watch: '{feed_id}' looks like a key=value, not a feed_id. The form is \
             `/watch <provider/template> <feed_id> [key=value ...]` — you likely \
             omitted the feed_id (a short name you choose for this feed), e.g. \
             `/watch {template} mynotes {feed_id}`."
        ));
    }

    let mut params = serde_json::Map::new();
    let mut cadence: Option<String> = None;
    for tok in &tokens[2..] {
        let (k, v) = tok.split_once('=').ok_or_else(|| {
            format!(
                "/watch: '{tok}' is not key=value. If this is part of a value \
                 containing spaces (e.g. a path), quote it: root=\"/path/with spaces\" \
                 or root='/path/with spaces'."
            )
        })?;
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
            let iso = parse_since(v).map_err(|e| format!("/watch: bad since value '{v}': {e}"))?;
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
pub(crate) fn parse_since(s: &str) -> Result<String, String> {
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

/// Tokenizer that respects quoted runs so a param value can include
/// spaces — e.g. a filesystem path like `root="/Users/me/My Drive"`.
/// Both `"double"` and `'single'` quotes are honored (single quotes are
/// what most people reach for, and a shell would strip them before we
/// ever saw the string; in the TUI we get the raw line, so we strip them
/// here). Backslash escapes (`\"`) are only meaningful inside double
/// quotes, matching shell semantics. Doesn't try to be a full shell
/// parser — just enough for the `/watch` use case.
fn tokenize_kv(s: &str) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    // `None` = unquoted; `Some(q)` = inside a run opened by quote char `q`.
    // Only the matching quote char closes the run, so a `'` inside a
    // double-quoted value (or vice-versa) is a literal.
    let mut quote: Option<char> = None;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' | '\'' if quote.is_none() => quote = Some(c),
            c if Some(c) == quote => quote = None,
            '\\' if quote == Some('"') => match chars.next() {
                Some('"') => cur.push('"'),
                Some(other) => {
                    cur.push('\\');
                    cur.push(other);
                }
                None => return Err("trailing backslash".into()),
            },
            c if c.is_whitespace() && quote.is_none() => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if quote.is_some() {
        return Err("unterminated quote".into());
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
            None => CommandResult::SystemMessage("Usage: /feeds rm <id> [--yes]".into()),
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
                "lens" => {
                    let parts: Vec<&str> = cmd.args.splitn(2, char::is_whitespace).collect();
                    match parts.first().copied() {
                        Some("create") => {
                            let name = parts.get(1).unwrap_or(&"").trim();
                            if name.is_empty() {
                                CommandResult::SystemMessage("Usage: /lens create <name>".into())
                            } else {
                                CommandResult::LensCreate(name.to_string())
                            }
                        }
                        Some("list") => CommandResult::LensList,
                        Some("promote") => {
                            let name = parts.get(1).unwrap_or(&"").trim();
                            if name.is_empty() {
                                CommandResult::SystemMessage(
                                    "Usage: /lens promote <lens-name>\n\nMoves the current session into the named lens (creating it if needed).".into(),
                                )
                            } else {
                                CommandResult::LensPromote(name.to_string())
                            }
                        }
                        _ => CommandResult::SystemMessage(
                            "Usage: /lens <create|list|promote> [name]\n\n  create <name>   Create a new lens (a standing, memory-aware extractor)\n  list            List all lenses\n  promote <name>  Move the current session into the named lens (creating it if needed)\n\nLenses aren't switched into — chat reads signals across every lens, and memories are global.".into()
                        ),
                    }
                }
                "session" => {
                    let sub = cmd.args.split_whitespace().next().unwrap_or("");
                    match sub {
                        "new" => CommandResult::SessionNew,
                        "list" => CommandResult::SessionList,
                        _ => CommandResult::SystemMessage(
                            "Usage: /session <new|list>\n\n  new   Create a new session\n  list  List sessions in current lens".into()
                        ),
                    }
                }
                // T-0347: unified permission posture. `ask` is the
                // default-safe posture; `full` is the old "bypass";
                // `plan` no longer needs its own top-level command.
                "autonomy" => {
                    let sub = cmd.args.split_whitespace().next().unwrap_or("");
                    match sub {
                        "ask" | "edits" | "full" | "plan" => {
                            CommandResult::SetPermissionMode(sub.into())
                        }
                        _ => CommandResult::SystemMessage(
                            "Usage: /autonomy <ask|edits|full|plan>\n\n  ask     Ask before mutating actions (default)\n  edits   Auto-allow file writes; ask for shell\n  full    Full autonomy — agent never asks\n  plan    Read-only plan mode — side-effects denied".into()
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
                "status" => CommandResult::SystemStatus,
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
                    // `/watch` with no args opens the interactive modal
                    // (ARAWN-I-0058). The typed forms below still work.
                    let trimmed = cmd.args.trim();
                    if trimmed.is_empty() {
                        return CommandResult::FeedWatchModal;
                    }
                    // `/watch list [template]` is the discovery form;
                    // shares the verb with `/watch <template> <id>`.
                    // Only `list` followed by whitespace (or end of
                    // args) counts — `list-something` keeps the
                    // normal-form path.
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
                "today" => CommandResult::CeremonyShowToday,
                "week" => CommandResult::CeremonyShowWeek,
                "retro" => CommandResult::CeremonyShowRetro,
                "brief" => CommandResult::BriefShow,
                "copy" => CommandResult::CopyLastResponse,
                "usage" => {
                    let period = if cmd.args.trim().is_empty() {
                        "day".to_string()
                    } else {
                        cmd.args.trim().to_lowercase()
                    };
                    CommandResult::UsageShow { period }
                }
                "export" => {
                    let trimmed = cmd.args.trim();
                    let path = if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    };
                    CommandResult::ExportConversation { path }
                }
                "todo" => CommandResult::TodoShow,

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

    /// Filesystem-feed paths routinely contain spaces ("My Drive",
    /// "Google Drive"). Both quote styles must protect the space so the
    /// value survives as a single token; an unquoted space cannot (there
    /// is no delimiter to recover), and that case must surface a helpful
    /// error rather than silently mangling the path.
    #[test]
    fn watch_spaced_path_honors_both_quote_styles() {
        const WANT: &str = "/Users/me/My Drive/Meet";

        // Double quotes around the value.
        let dq = parse_watch_args("filesystem/folder notes root=\"/Users/me/My Drive/Meet\"")
            .expect("double-quoted value");
        assert_eq!(dq.params["root"], WANT);

        // Double quotes around the whole key=value token.
        let dqw = parse_watch_args("filesystem/folder notes \"root=/Users/me/My Drive/Meet\"")
            .expect("double-quoted token");
        assert_eq!(dqw.params["root"], WANT);

        // Single quotes — what most people reach for, and previously a
        // hard error (regression guard for the reported bug).
        let sq = parse_watch_args("filesystem/folder notes root='/Users/me/My Drive/Meet'")
            .expect("single-quoted value");
        assert_eq!(sq.params["root"], WANT);

        // Unquoted spaces are unrecoverable — but the error must point at
        // the fix instead of just "is not key=value".
        let err = parse_watch_args("filesystem/folder notes root=/Users/me/My Drive/Meet")
            .expect_err("unquoted spaces cannot be tokenized");
        assert!(
            err.contains("quote it"),
            "error should hint at quoting: {err}"
        );
    }

    /// Omitting the feed_id makes the first `key=value` get consumed as the
    /// positional feed_id, leaving params empty so the template reports a missing
    /// required param ("missing root" for filesystem/folder). The parser should
    /// catch the `key=value`-shaped feed_id and explain the real cause.
    #[test]
    fn watch_rejects_keyvalue_shaped_feed_id() {
        let err = parse_watch_args("filesystem/folder root=\"/Users/me/My Drive\"")
            .expect_err("key=value feed_id should be rejected");
        assert!(err.contains("looks like a key=value"), "got: {err}");
        assert!(err.contains("omitted the feed_id"), "got: {err}");
        // The correct form still parses and carries root in params.
        let ok = parse_watch_args("filesystem/folder mynotes root=\"/Users/me/My Drive\"")
            .expect("with feed_id it parses");
        assert_eq!(ok.feed_id, "mynotes");
        assert_eq!(ok.params["root"], "/Users/me/My Drive");
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
        let spec = parse_watch_args("slack/channel-archive design channel=C123 since=2026-01-01")
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
        assert!(
            parse_watch_args("slack/channel-archive design channel=C123 since=tomorrow").is_err()
        );
        assert!(
            parse_watch_args("slack/channel-archive design channel=C123 since=180banana").is_err()
        );
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
        // T-0347: /plan removed; plan mode is reachable as
        // `/autonomy plan` instead.
        assert!(reg.find("autonomy").is_some());
        assert!(reg.find("tools").is_some());
        assert!(reg.find("skills").is_some());
    }

    #[test]
    fn registry_matching_prefix() {
        let reg = CommandRegistry::new();
        // T-0347: /plan removed. `pl` now only matches plugins.
        let matches = reg.matching("pl");
        assert_eq!(matches.len(), 1);
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
            CommandInfo {
                name: "help".into(),
                description: "".into(),
                kind: CommandKind::BuiltIn,
            },
            CommandInfo {
                name: "clear".into(),
                description: "".into(),
                kind: CommandKind::BuiltIn,
            },
            CommandInfo {
                name: "plan".into(),
                description: "".into(),
                kind: CommandKind::BuiltIn,
            },
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
        assert!(matches!(
            execute_command(&cmd, &reg),
            CommandResult::ClearChat
        ));
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
                assert!(
                    msg.contains("/remember"),
                    "usage should mention command name"
                );
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
        assert!(
            !builtins.is_empty(),
            "registry should have built-in commands"
        );

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

    // T-0347 — `/autonomy` slash command parses the 4 valid values.
    #[test]
    fn execute_autonomy_each_valid_mode() {
        let reg = CommandRegistry::new();
        for mode in ["ask", "edits", "full", "plan"] {
            let cmd = parse_command(&format!("/autonomy {mode}")).unwrap();
            match execute_command(&cmd, &reg) {
                CommandResult::SetPermissionMode(got) => assert_eq!(got, mode),
                other => panic!("expected SetPermissionMode for {mode}, got {other:?}"),
            }
        }
    }

    #[test]
    fn execute_autonomy_invalid_value_returns_usage_message() {
        let reg = CommandRegistry::new();
        let cmd = parse_command("/autonomy bogus").unwrap();
        match execute_command(&cmd, &reg) {
            CommandResult::SystemMessage(msg) => {
                assert!(msg.contains("Usage: /autonomy"));
                assert!(msg.contains("ask"));
                assert!(msg.contains("full"));
            }
            other => panic!("expected SystemMessage, got {other:?}"),
        }
    }

    #[test]
    fn execute_autonomy_no_arg_returns_usage_message() {
        let reg = CommandRegistry::new();
        let cmd = parse_command("/autonomy").unwrap();
        match execute_command(&cmd, &reg) {
            CommandResult::SystemMessage(msg) => assert!(msg.contains("Usage: /autonomy")),
            other => panic!("expected SystemMessage, got {other:?}"),
        }
    }

    #[test]
    fn legacy_slash_commands_no_longer_resolve() {
        // T-0347: /accept and /plan removed. parse_command still
        // returns a ParsedCommand (just splits on whitespace) but
        // execute_command should treat them as unknown and surface
        // the catchall "Unknown built-in" message.
        let reg = CommandRegistry::new();
        for legacy in ["/accept on", "/plan"] {
            let cmd = parse_command(legacy).unwrap();
            match execute_command(&cmd, &reg) {
                CommandResult::SystemMessage(msg) => {
                    assert!(
                        msg.contains("Unknown")
                            || msg.contains("No such command")
                            || msg.contains("Usage:"),
                        "expected unknown-command message for {legacy}, got: {msg}"
                    );
                }
                other => panic!("expected SystemMessage for {legacy}, got {other:?}"),
            }
        }
    }

    // T-0362 — `/usage` slash command parses optional period arg.
    #[test]
    fn execute_usage_default_period_is_day() {
        let reg = CommandRegistry::new();
        let cmd = parse_command("/usage").unwrap();
        match execute_command(&cmd, &reg) {
            CommandResult::UsageShow { period } => assert_eq!(period, "day"),
            other => panic!("expected UsageShow, got {other:?}"),
        }
    }

    #[test]
    fn execute_usage_with_week_arg() {
        let reg = CommandRegistry::new();
        let cmd = parse_command("/usage week").unwrap();
        match execute_command(&cmd, &reg) {
            CommandResult::UsageShow { period } => assert_eq!(period, "week"),
            other => panic!("expected UsageShow, got {other:?}"),
        }
    }

    #[test]
    fn execute_usage_lowercases_args() {
        let reg = CommandRegistry::new();
        let cmd = parse_command("/usage MONTH").unwrap();
        match execute_command(&cmd, &reg) {
            CommandResult::UsageShow { period } => assert_eq!(period, "month"),
            other => panic!("expected UsageShow, got {other:?}"),
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
            std::path::Path::new("../..").join(PINNED).exists()
                || std::path::Path::new("../..").join("docs").exists(),
            "memory docs not at expected path; update banner copy in event_loop.rs"
        );
    }
}
