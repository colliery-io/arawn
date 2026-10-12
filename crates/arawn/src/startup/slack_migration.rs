//! One-time move of a single-workspace Slack install to one integration
//! per workspace (COLLIERY-I-0520, ARAWN-T-0524). Runs at `arawn serve`
//! startup, before the integrations, permissions and feeds load.
//!
//! When the token store still has the pre-workspace key `slack`:
//!
//! 1. the alias is chosen from the token's team (or `auth.test`, or
//!    `default`) and written to `<data>/slack-migration.json`, so a crash
//!    between steps resumes with the same alias;
//! 2. the token moves to `slack@<alias>`, and `<data>/integrations/slack/`
//!    to `slack@<alias>/` if it exists;
//! 3. every `slack/*` feed without a `workspace` param gets `<alias>` (the
//!    row and its `meta.json`), and their projection rows get the
//!    `workspace` column;
//! 4. permission rules that name an old tool (`slack_post`,
//!    `slack_history(...)`) are rewritten in the given arawn.toml files:
//!    `allow` to the alias's tool (`slack_<alias>_<op>`), `deny` and `ask`
//!    to every workspace's (`slack_*_<op>`), so a workspace connected
//!    later is held by the same rule. Each file is each backed up first next to itself; in the
//!    same files a `[[feeds]]` Slack entry gets `workspace` in its params,
//!    so it still matches the bound feed. These are the only files
//!    written outside the data dir;
//! 5. the marker is set to done.
//!
//! Each step checks before it writes, so a second run changes nothing.

use std::path::{Path, PathBuf};

use arawn_auth::{Token, TokenStore};
use arawn_integrations::slack::{
    SLACK_TOOL_OPS, SlackTeam, connected_aliases, pick_alias, team_of, tool_name, workspace_name,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// The pre-workspace token-store key.
const OLD_KEY: &str = "slack";

/// `<data>/slack-migration.json`: the alias chosen, and whether every
/// step is done.
pub const MARKER_FILE: &str = "slack-migration.json";

/// Suffix of the backup written next to a rewritten settings file.
pub const BACKUP_SUFFIX: &str = "pre-slack-workspaces";

#[derive(Debug, Serialize, Deserialize)]
struct Marker {
    alias: String,
    done: bool,
}

/// Finds the team of an old token that does not store one.
#[async_trait]
pub trait TeamLookup: Send + Sync {
    async fn team(&self, token: &Token) -> Option<SlackTeam>;
}

/// Production lookup: Slack's `auth.test` with the token.
pub struct AuthTest;

#[async_trait]
impl TeamLookup for AuthTest {
    async fn team(&self, token: &Token) -> Option<SlackTeam> {
        let ctx = arawn_integrations::slack::build_slack_client(token);
        let resp = ctx.session().auth_test().await.ok()?;
        Some(SlackTeam {
            id: resp.team_id.to_string(),
            name: resp.team,
        })
    }
}

/// What a run did.
#[derive(Debug, Default, PartialEq)]
pub struct MigrationReport {
    pub alias: String,
    pub token_moved: bool,
    pub dir_moved: bool,
    /// Feed ids that got `workspace`.
    pub feeds_bound: Vec<String>,
    /// Projection rows that got the `workspace` column.
    pub rows_filled: usize,
    /// `(file, old rule, new rule)`.
    pub rules_rewritten: Vec<(PathBuf, String, String)>,
    /// `[[feeds]]` ids in arawn.toml that got `workspace`.
    pub declared_feeds_bound: Vec<String>,
    /// Backups written, one per rewritten file.
    pub backups: Vec<PathBuf>,
}

impl MigrationReport {
    /// The text of the startup notice.
    pub fn notice(&self) -> String {
        let mut s = format!(
            "Slack moved to one integration per workspace: your workspace is now \
             slack@{alias} and its tools are slack_{tools}_*.",
            alias = self.alias,
            tools = self.alias.replace('-', "_"),
        );
        if !self.feeds_bound.is_empty() {
            s.push_str(&format!(
                " Feeds bound to it: {}.",
                self.feeds_bound.join(", ")
            ));
        }
        if !self.backups.is_empty() {
            let files: Vec<String> = self
                .backups
                .iter()
                .map(|b| b.display().to_string())
                .collect();
            let rules: Vec<String> = self
                .rules_rewritten
                .iter()
                .map(|(_, old, new)| format!("{old} -> {new}"))
                .collect();
            let mut changes = Vec::new();
            if !rules.is_empty() {
                changes.push(format!("permission rules {}", rules.join(", ")));
            }
            if !self.declared_feeds_bound.is_empty() {
                changes.push(format!(
                    "[[feeds]] {} got workspace = \"{}\"",
                    self.declared_feeds_bound.join(", "),
                    self.alias
                ));
            }
            s.push_str(&format!(
                " Changed in arawn.toml (the only change outside the data dir): {}. \
                 deny and ask rules now cover every Slack workspace; allow rules only \
                 slack@{}. Backup: {}.",
                changes.join("; "),
                self.alias,
                files.join(", ")
            ));
        }
        s
    }
}

/// Run the migration. `Ok(None)` when there is nothing to do: a new
/// install, or one already migrated. `settings_files` are the arawn.toml
/// files whose `[permissions]` rules are rewritten.
pub async fn migrate(
    data_dir: &Path,
    settings_files: &[PathBuf],
    lookup: &dyn TeamLookup,
) -> Result<Option<MigrationReport>, String> {
    let marker_path = data_dir.join(MARKER_FILE);
    let marker: Option<Marker> = match std::fs::read_to_string(&marker_path) {
        Ok(s) => Some(serde_json::from_str(&s).map_err(|e| format!("{MARKER_FILE}: {e}"))?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(format!("{MARKER_FILE}: {e}")),
    };
    if marker.as_ref().is_some_and(|m| m.done) {
        return Ok(None);
    }
    let store = TokenStore::open(data_dir).map_err(|e| e.to_string())?;
    let old = store.load(OLD_KEY).map_err(|e| e.to_string())?;

    // 1. The alias: chosen once, then read back on a resumed run.
    let alias = match (&marker, &old) {
        (Some(m), _) => m.alias.clone(),
        (None, None) => return Ok(None),
        (None, Some(token)) => {
            let team = match team_of(token) {
                Some(t) => Some(t),
                None => lookup.team(token).await,
            };
            let existing: Vec<(String, Option<SlackTeam>)> = connected_aliases(data_dir)
                .into_iter()
                .map(|a| {
                    let t = store
                        .load(&workspace_name(&a))
                        .ok()
                        .flatten()
                        .and_then(|t| team_of(&t));
                    (a, t)
                })
                .collect();
            let alias = pick_alias(team.as_ref(), &existing);
            write_marker(&marker_path, &alias, false)?;
            alias
        }
    };
    let mut report = MigrationReport {
        alias: alias.clone(),
        ..Default::default()
    };
    let new_key = workspace_name(&alias);

    // 2. Token, then the credentials dir.
    if let Some(token) = &old {
        if store.load(&new_key).map_err(|e| e.to_string())?.is_none() {
            store.save(&new_key, token).map_err(|e| e.to_string())?;
        }
        store.delete(OLD_KEY).map_err(|e| e.to_string())?;
        report.token_moved = true;
    }
    let old_dir = data_dir.join("integrations").join(OLD_KEY);
    let new_dir = data_dir.join("integrations").join(&new_key);
    if old_dir.is_dir() && !new_dir.exists() {
        std::fs::rename(&old_dir, &new_dir)
            .map_err(|e| format!("move {}: {e}", old_dir.display()))?;
        report.dir_moved = true;
    }

    // 3. Feeds and their rows.
    let slack_feeds = bind_feeds(data_dir, &alias, &mut report)?;
    let projections = data_dir.join("projections.db");
    if projections.exists() && !slack_feeds.is_empty() {
        let p =
            arawn_projections::ProjectionStore::open(&projections).map_err(|e| e.to_string())?;
        for id in &slack_feeds {
            report.rows_filled += arawn_projections::slack::set_workspace(&p, id, &alias)
                .map_err(|e| e.to_string())?;
        }
    }

    // 4. Permission rules and [[feeds]] in arawn.toml.
    for file in settings_files {
        rewrite_settings(file, &alias, &mut report)?;
    }

    // 5. Done.
    write_marker(&marker_path, &alias, true)?;
    Ok(Some(report))
}

fn write_marker(path: &Path, alias: &str, done: bool) -> Result<(), String> {
    let body = serde_json::to_string_pretty(&Marker {
        alias: alias.to_string(),
        done,
    })
    .map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, body).map_err(|e| format!("write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("write {}: {e}", path.display()))
}

/// Give every `slack/*` feed with no `workspace` the alias, in its row
/// and its `meta.json`. Returns the ids of all feeds on the alias.
fn bind_feeds(
    data_dir: &Path,
    alias: &str,
    report: &mut MigrationReport,
) -> Result<Vec<String>, String> {
    let db = data_dir.join("arawn.db");
    if !db.exists() {
        return Ok(Vec::new());
    }
    let conn = rusqlite::Connection::open(&db).map_err(|e| format!("open arawn.db: {e}"))?;
    let feeds = arawn_feeds::FeedStore::new(&conn);
    let layout = arawn_feeds::DataLayout::new(data_dir);
    let mut on_alias = Vec::new();
    for rec in feeds.list_all().map_err(|e| e.to_string())? {
        if !rec.template.starts_with("slack/") {
            continue;
        }
        let mut params = rec.params.0.clone();
        if params.get("workspace").is_none() {
            if !params.is_object() {
                params = serde_json::json!({});
            }
            params["workspace"] = serde_json::json!(alias);
            feeds
                .set_params(&rec.id, &arawn_feeds::TemplateParams::new(params.clone()))
                .map_err(|e| e.to_string())?;
            report.feeds_bound.push(rec.id.clone());
        }
        // meta.json: the projection walker reads the workspace there.
        if let Ok(dir) = layout.feed_dir(&rec.template, &rec.id)
            && let Ok(Some(mut meta)) = arawn_feeds::MetaStore::read(&dir)
            && meta.params.0.get("workspace").is_none()
        {
            meta.params = arawn_feeds::TemplateParams::new(params.clone());
            arawn_feeds::MetaStore::write(&dir, &meta).map_err(|e| e.to_string())?;
        }
        if params.get("workspace").and_then(|v| v.as_str()) == Some(alias) {
            on_alias.push(rec.id);
        }
    }
    Ok(on_alias)
}

/// Which tools a rewritten rule names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleScope {
    /// The migrated workspace's tool, `slack_<alias>_<op>` (allow).
    Alias,
    /// Every workspace's, `slack_*_<op>` (deny, ask): a workspace
    /// connected later must not escape a deny or an ask.
    AnyWorkspace,
}

/// `slack_<op>` or `slack_<op>(content)` → the new tool name for
/// `scope`, keeping the content pattern. `None` when the rule names no
/// old Slack tool (globs such as `slack_*` already match the new names).
pub fn rewrite_rule(spec: &str, alias: &str, scope: RuleScope) -> Option<String> {
    let (tool, rest) = match spec.find('(') {
        Some(i) => spec.split_at(i),
        None => (spec, ""),
    };
    let op = tool.strip_prefix("slack_")?;
    if !SLACK_TOOL_OPS.contains(&op) {
        return None;
    }
    let tool = match scope {
        RuleScope::Alias => tool_name(alias, op),
        RuleScope::AnyWorkspace => format!("slack_*_{op}"),
    };
    Some(format!("{tool}{rest}"))
}

/// Rewrite the old Slack tool names in `[permissions]` allow / deny / ask
/// of one arawn.toml, and give each Slack `[[feeds]]` entry `workspace`.
/// Writes a backup first, and only when something changes. A missing
/// file is skipped.
fn rewrite_settings(file: &Path, alias: &str, report: &mut MigrationReport) -> Result<(), String> {
    if !file.exists() {
        return Ok(());
    }
    let mut doc = crate::startup::setup::edit::load_doc(file).map_err(|e| format!("{e:#}"))?;
    let mut changed = Vec::new();
    if let Some(perms) = doc
        .get_mut("permissions")
        .and_then(|p| p.as_table_like_mut())
    {
        for list in ["allow", "deny", "ask"] {
            let Some(arr) = perms.get_mut(list).and_then(|v| v.as_array_mut()) else {
                continue;
            };
            let scope = if list == "allow" {
                RuleScope::Alias
            } else {
                RuleScope::AnyWorkspace
            };
            for v in arr.iter_mut() {
                let Some(new) = v.as_str().and_then(|s| rewrite_rule(s, alias, scope)) else {
                    continue;
                };
                let old = v.as_str().unwrap_or_default().to_string();
                let decor = v.decor().clone();
                *v = toml_edit::Value::from(new.as_str());
                *v.decor_mut() = decor;
                changed.push((file.to_path_buf(), old, new));
            }
        }
    }
    let mut feeds_bound = Vec::new();
    if let Some(feeds) = doc
        .get_mut("feeds")
        .and_then(|f| f.as_array_of_tables_mut())
    {
        for t in feeds.iter_mut() {
            let slack = t
                .get("template")
                .and_then(|v| v.as_str())
                .is_some_and(|tpl| tpl.starts_with("slack/"));
            if !slack {
                continue;
            }
            let id = t
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string();
            if !t.contains_key("params") {
                t.insert("params", toml_edit::value(toml_edit::InlineTable::new()));
            }
            let Some(params) = t.get_mut("params").and_then(|p| p.as_table_like_mut()) else {
                continue;
            };
            if !params.contains_key("workspace") {
                params.insert("workspace", toml_edit::value(alias));
                feeds_bound.push(id);
            }
        }
    }
    if changed.is_empty() && feeds_bound.is_empty() {
        return Ok(());
    }
    let backup = PathBuf::from(format!("{}.{BACKUP_SUFFIX}", file.display()));
    if !backup.exists() {
        std::fs::copy(file, &backup).map_err(|e| format!("back up {}: {e}", file.display()))?;
    }
    crate::startup::setup::edit::write_config(file, &doc).map_err(|e| format!("{e:#}"))?;
    report.backups.push(backup);
    report.rules_rewritten.extend(changed);
    report.declared_feeds_bound.extend(feeds_bound);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct NoLookup;
    #[async_trait]
    impl TeamLookup for NoLookup {
        async fn team(&self, _: &Token) -> Option<SlackTeam> {
            None
        }
    }

    struct Lookup(&'static str, &'static str);
    #[async_trait]
    impl TeamLookup for Lookup {
        async fn team(&self, _: &Token) -> Option<SlackTeam> {
            Some(SlackTeam {
                id: self.0.into(),
                name: self.1.into(),
            })
        }
    }

    fn token(team: Option<(&str, &str)>) -> Token {
        let mut extras = serde_json::Map::new();
        if let Some((id, name)) = team {
            extras.insert("team".into(), json!({ "id": id, "name": name }));
        }
        Token {
            access: "xoxb-old".into(),
            refresh: None,
            expires_at: None,
            scope: Some("channels:read,chat:write".into()),
            token_type: "Bearer".into(),
            extras,
        }
    }

    const SETTINGS: &str = r#"# my settings
[engine]
llm = "default"

[permissions]
allow = ["slack_history", "slack_list_channels", "Read"]
deny = ["slack_post(#general)"]
ask = [
  "slack_react",  # ask before reacting
  "slack_*",
  "slack_open_dm_extra",
]

[[feeds]]
id = "slack-mentions"
template = "slack/my-mentions"

[[feeds]]
id = "gmail-inbox"
template = "gmail/inbox-archive"
"#;

    /// The old layout: a `slack` token, a creds dir, a mentions feed
    /// under the old default id with no params, a channel feed, rows
    /// written before the workspace column, and permission rules.
    fn old_install(team: Option<(&str, &str)>) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        TokenStore::open(data)
            .unwrap()
            .save("slack", &token(team))
            .unwrap();
        std::fs::create_dir_all(data.join("integrations/slack")).unwrap();
        std::fs::write(data.join("integrations/slack/note"), "x").unwrap();

        arawn_storage::Store::open(data).unwrap();
        let conn = rusqlite::Connection::open(data.join("arawn.db")).unwrap();
        let feeds = arawn_feeds::FeedStore::new(&conn);
        let layout = arawn_feeds::DataLayout::new(data);
        for (id, template, params) in [
            (
                "slack-mentions",
                "slack/my-mentions",
                serde_json::Value::Null,
            ),
            (
                "design",
                "slack/channel-archive",
                json!({ "channel": "C1" }),
            ),
            (
                "gmail-inbox",
                "gmail/inbox-archive",
                serde_json::Value::Null,
            ),
        ] {
            let params = arawn_feeds::TemplateParams::new(params);
            feeds
                .insert(&arawn_feeds::new_record(
                    id,
                    template,
                    params.clone(),
                    "*/15 * * * *",
                ))
                .unwrap();
            let fdir = layout.ensure_feed_dir(template, id).unwrap();
            arawn_feeds::MetaStore::write(
                &fdir,
                &arawn_feeds::FeedMeta::new(template, params, serde_json::Value::Null),
            )
            .unwrap();
        }

        let p = arawn_projections::ProjectionStore::open(&data.join("projections.db")).unwrap();
        let msg = |ts: &str| arawn_projections::slack::SlackMessageProjection {
            id: format!("sl-{ts}"),
            feed_id: "slack-mentions".into(),
            source_id: ts.into(),
            source_ts: chrono::Utc::now(),
            channel_id: Some("C1".into()),
            sender_id: Some("U1".into()),
            text: "hello".into(),
            thread_ts: None,
            reactions: vec![],
            is_thread_reply: false,
            workspace: None,
        };
        p.write_batch(&[msg("1.1"), msg("1.2")]).unwrap();

        let settings = data.join("arawn.toml");
        std::fs::write(&settings, SETTINGS).unwrap();
        (dir, settings)
    }

    fn feed_params(data: &Path, id: &str) -> serde_json::Value {
        let conn = rusqlite::Connection::open(data.join("arawn.db")).unwrap();
        arawn_feeds::FeedStore::new(&conn)
            .get(id)
            .unwrap()
            .unwrap()
            .params
            .0
    }

    fn workspaces(data: &Path) -> Vec<Option<String>> {
        let p = arawn_projections::ProjectionStore::open(&data.join("projections.db")).unwrap();
        p.ensure_feed_type("slack_messages").unwrap();
        let conn = p.conn().lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT workspace FROM slack_messages")
            .unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }

    #[tokio::test]
    async fn old_layout_moves_to_the_team_alias_and_a_second_run_changes_nothing() {
        let (dir, settings) = old_install(Some(("T1", "Acme Corp")));
        let data = dir.path();
        let report = migrate(data, std::slice::from_ref(&settings), &NoLookup)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(report.alias, "acme-corp");
        assert!(report.token_moved && report.dir_moved);

        // Connected under the new alias, with the same token.
        let store = TokenStore::open(data).unwrap();
        assert!(store.load("slack").unwrap().is_none());
        assert_eq!(
            store.load("slack@acme-corp").unwrap().unwrap().access,
            "xoxb-old"
        );
        assert!(data.join("integrations/slack@acme-corp/note").exists());
        assert!(!data.join("integrations/slack").exists());

        // Feeds bound, in the row and in meta.json; other feeds untouched.
        assert_eq!(report.feeds_bound, ["design", "slack-mentions"]);
        assert_eq!(
            feed_params(data, "slack-mentions"),
            json!({ "workspace": "acme-corp" })
        );
        assert_eq!(
            feed_params(data, "design"),
            json!({ "channel": "C1", "workspace": "acme-corp" })
        );
        assert!(feed_params(data, "gmail-inbox").is_null());
        let meta_dir = arawn_feeds::DataLayout::new(data)
            .feed_dir("slack/my-mentions", "slack-mentions")
            .unwrap();
        let meta = arawn_feeds::MetaStore::read(&meta_dir).unwrap().unwrap();
        assert_eq!(meta.params.0["workspace"], "acme-corp");

        // Rows back-filled.
        assert_eq!(report.rows_filled, 2);
        assert_eq!(
            workspaces(data),
            [Some("acme-corp".to_string()), Some("acme-corp".to_string())]
        );

        // Rules rewritten, none lost; comments and other rules kept.
        let text = std::fs::read_to_string(&settings).unwrap();
        let p = arawn_engine::permissions::load_permissions_from_file(&settings);
        assert_eq!(
            p.allow,
            [
                "slack_acme_corp_history",
                "slack_acme_corp_list_channels",
                "Read"
            ]
        );
        assert_eq!(p.deny, ["slack_*_post(#general)"]);
        assert_eq!(p.ask, ["slack_*_react", "slack_*", "slack_open_dm_extra"]);
        assert!(text.starts_with("# my settings\n"), "{text}");
        assert!(text.contains("# ask before reacting"), "{text}");
        assert_eq!(report.rules_rewritten.len(), 4);
        // The declared feed still matches the bound one.
        let cfg: crate::ArawnConfig = toml::from_str(&text).unwrap();
        assert_eq!(report.declared_feeds_bound, ["slack-mentions"]);
        assert_eq!(cfg.feeds[0].params["workspace"].as_str(), Some("acme-corp"));
        assert!(cfg.feeds[1].params.is_empty());
        let backup = PathBuf::from(format!("{}.{BACKUP_SUFFIX}", settings.display()));
        assert_eq!(report.backups, [backup.clone()]);
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), SETTINGS);
        let notice = report.notice();
        assert!(notice.contains("slack@acme-corp"), "{notice}");
        assert!(notice.contains("slack_post(#general) -> slack_*_post(#general)"));
        assert!(notice.contains("deny and ask rules now cover every Slack workspace"));
        assert!(notice.contains("outside the data dir"), "{notice}");

        // A second run changes nothing.
        let before = std::fs::read_to_string(&settings).unwrap();
        assert!(
            migrate(data, std::slice::from_ref(&settings), &NoLookup)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(std::fs::read_to_string(&settings).unwrap(), before);
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), SETTINGS);
        assert!(store.load("slack@acme-corp").unwrap().is_some());
    }

    #[tokio::test]
    async fn no_stored_team_asks_slack_and_else_uses_default() {
        let (dir, settings) = old_install(None);
        let r = migrate(dir.path(), &[settings], &Lookup("T9", "Personal"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(r.alias, "personal");

        let (dir, settings) = old_install(None);
        let r = migrate(dir.path(), &[settings], &NoLookup)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(r.alias, "default");
    }

    #[tokio::test]
    async fn a_crash_after_the_token_moved_resumes_with_the_same_alias() {
        let (dir, settings) = old_install(Some(("T1", "Acme")));
        let data = dir.path();
        // Steps 1 and 2 ran, then the process died.
        write_marker(&data.join(MARKER_FILE), "acme", false).unwrap();
        let store = TokenStore::open(data).unwrap();
        let old = store.load("slack").unwrap().unwrap();
        store.save("slack@acme", &old).unwrap();
        store.delete("slack").unwrap();

        let r = migrate(data, std::slice::from_ref(&settings), &NoLookup)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(r.alias, "acme");
        assert!(!r.token_moved);
        assert_eq!(feed_params(data, "slack-mentions")["workspace"], "acme");
        let text = std::fs::read_to_string(&settings).unwrap();
        assert!(text.contains("\"slack_*_post(#general)\""), "{text}");
    }

    #[tokio::test]
    async fn a_new_install_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        assert!(migrate(dir.path(), &[], &NoLookup).await.unwrap().is_none());
        assert!(!dir.path().join(MARKER_FILE).exists());
    }

    #[test]
    fn rewrite_rule_cases() {
        use RuleScope::{Alias, AnyWorkspace};
        assert_eq!(
            rewrite_rule("slack_post", "acme", Alias).as_deref(),
            Some("slack_acme_post")
        );
        assert_eq!(
            rewrite_rule("slack_post", "acme", AnyWorkspace).as_deref(),
            Some("slack_*_post")
        );
        assert_eq!(
            rewrite_rule("slack_history(C1*)", "a-b", Alias).as_deref(),
            Some("slack_a_b_history(C1*)")
        );
        assert_eq!(
            rewrite_rule("slack_history(C1*)", "a-b", AnyWorkspace).as_deref(),
            Some("slack_*_history(C1*)")
        );
        for scope in [Alias, AnyWorkspace] {
            assert_eq!(rewrite_rule("slack_*", "acme", scope), None);
            assert_eq!(rewrite_rule("slack_*_post", "acme", scope), None);
            assert_eq!(rewrite_rule("slack_acme_post", "acme", scope), None);
            assert_eq!(rewrite_rule("Read", "acme", scope), None);
        }
    }

    #[test]
    fn a_workspace_glob_matches_only_its_op_in_every_workspace() {
        use arawn_engine::permissions::RuleKind;
        // Aliases have `-`, so tool names have `_` inside the alias part:
        // the glob still keys on the op at the end.
        for op in SLACK_TOOL_OPS {
            let rule =
                arawn_engine::PermissionRule::parse(RuleKind::Deny, &format!("slack_*_{op}"));
            for alias in ["acme", "acme-corp", "a-b-c", "post", "users"] {
                for other in SLACK_TOOL_OPS {
                    let name = tool_name(alias, other);
                    assert_eq!(
                        rule.matches(&name, "{}"),
                        op == other,
                        "slack_*_{op} vs {name}"
                    );
                }
            }
        }
        // The content part still applies.
        let rule = arawn_engine::PermissionRule::parse(RuleKind::Deny, "slack_*_post(#general)");
        assert!(rule.matches("slack_acme_corp_post", "#general"));
        assert!(!rule.matches("slack_acme_corp_post", "#random"));
    }
}
