//! In-place edits of `arawn.toml` for `arawn setup` (ARAWN-T-0501).
//!
//! Uses `toml_edit` so comments, key order and unrelated tables survive.
//! Every write is validated by parsing the result into [`ArawnConfig`]
//! first; an edit that would not load is never written.

use std::path::Path;

use anyhow::{Context, Result};
use toml_edit::{DocumentMut, Item, Table, value};

use crate::ArawnConfig;

/// Read `arawn.toml` as an editable document. A missing file is an
/// empty document.
pub fn load_doc(path: &Path) -> Result<DocumentMut> {
    match std::fs::read_to_string(path) {
        Ok(s) => s
            .parse::<DocumentMut>()
            .with_context(|| format!("{} is not valid TOML", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(DocumentMut::new()),
        Err(e) => Err(e).with_context(|| format!("cannot read {}", path.display())),
    }
}

/// `[integrations.<key>]`, created when absent. `[integrations]` itself
/// is implicit, so no empty `[integrations]` header is written.
fn integration_table<'a>(doc: &'a mut DocumentMut, key: &str) -> Result<&'a mut Table> {
    let root = doc.as_table_mut();
    if !root.contains_key("integrations") {
        let mut t = Table::new();
        t.set_implicit(true);
        root.insert("integrations", Item::Table(t));
    }
    let integrations = root["integrations"]
        .as_table_mut()
        .context("`integrations` in arawn.toml is not a table")?;
    if !integrations.contains_key(key) {
        integrations.insert(key, Item::Table(Table::new()));
    }
    integrations[key]
        .as_table_mut()
        .with_context(|| format!("`integrations.{key}` in arawn.toml is not a table"))
}

/// Set an OAuth client. With `client_secret: None` the secret is removed
/// from the file (it is expected from the env var instead).
pub fn set_oauth_client(
    doc: &mut DocumentMut,
    key: &str,
    client_id: &str,
    client_secret: Option<&str>,
) -> Result<()> {
    let t = integration_table(doc, key)?;
    t["client_id"] = value(client_id);
    match client_secret {
        Some(s) => t["client_secret"] = value(s),
        None => {
            t.remove("client_secret");
        }
    }
    Ok(())
}

/// Set the GitHub App fields.
pub fn set_github_app(
    doc: &mut DocumentMut,
    app_id: &str,
    app_slug: &str,
    private_key_path: &str,
) -> Result<()> {
    let t = integration_table(doc, "github")?;
    t["app_id"] = value(app_id);
    t["app_slug"] = value(app_slug);
    t["private_key_path"] = value(private_key_path);
    Ok(())
}

/// Parse the document as arawn would load it.
pub fn validate(doc: &DocumentMut) -> Result<ArawnConfig> {
    toml::from_str::<ArawnConfig>(&doc.to_string())
        .context("internal error: the edited arawn.toml does not load")
}

/// Validate, then write atomically (temp file + rename). The file can
/// hold client secrets, so on Unix it is written with mode 0600.
pub fn write_config(path: &Path, doc: &DocumentMut) -> Result<ArawnConfig> {
    let cfg = validate(doc)?;
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir)?;
    let tmp = tempfile::NamedTempFile::new_in(dir)?;
    std::fs::write(tmp.path(), doc.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o600))?;
    }
    tmp.persist(path)
        .with_context(|| format!("cannot write {}", path.display()))?;
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXISTING: &str = r#"# my arawn config
[llm.default]
provider = "groq" # fast
model = "openai/gpt-oss-120b"

[engine]
llm = "default"
"#;

    #[test]
    fn adds_oauth_block_and_keeps_comments_and_other_tables() {
        let mut doc: DocumentMut = EXISTING.parse().unwrap();
        set_oauth_client(&mut doc, "google", "gid", Some("gsec")).unwrap();
        let out = doc.to_string();
        assert!(out.starts_with("# my arawn config\n"));
        assert!(out.contains("provider = \"groq\" # fast"));
        assert!(out.contains("[integrations.google]"));
        assert!(
            !out.contains("[integrations]\n"),
            "no bare [integrations] header:\n{out}"
        );
        let cfg = validate(&doc).unwrap();
        assert_eq!(cfg.integrations.google.client_id, "gid");
        assert_eq!(cfg.integrations.google.client_secret, "gsec");
        assert_eq!(cfg.engine.llm, "default");
    }

    #[test]
    fn replaces_existing_values_and_removes_secret_for_env_mode() {
        let mut doc: DocumentMut = r#"
[integrations.slack]
client_id = "old"
client_secret = "oldsec"
"#
        .parse()
        .unwrap();
        set_oauth_client(&mut doc, "slack", "new", None).unwrap();
        let cfg = validate(&doc).unwrap();
        assert_eq!(cfg.integrations.slack.client_id, "new");
        assert_eq!(cfg.integrations.slack.client_secret, "");
        assert!(!doc.to_string().contains("oldsec"));
    }

    #[test]
    fn sets_github_app_fields() {
        let mut doc = DocumentMut::new();
        set_github_app(&mut doc, "42", "arawn-me", "/keys/app.pem").unwrap();
        let cfg = validate(&doc).unwrap();
        assert_eq!(cfg.integrations.github.app_id, "42");
        assert_eq!(cfg.integrations.github.app_slug, "arawn-me");
        assert_eq!(cfg.integrations.github.private_key_path, "/keys/app.pem");
    }

    #[test]
    fn rejects_non_table_integrations_key() {
        let mut doc: DocumentMut = "integrations = 5\n".parse().unwrap();
        assert!(set_oauth_client(&mut doc, "google", "a", Some("b")).is_err());
    }

    #[test]
    fn write_config_round_trips_and_is_private() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("arawn.toml");
        std::fs::write(&path, EXISTING).unwrap();
        let mut doc = load_doc(&path).unwrap();
        set_oauth_client(&mut doc, "atlassian", "aid", Some("asec")).unwrap();
        write_config(&path, &doc).unwrap();
        let reread = std::fs::read_to_string(&path).unwrap();
        assert!(reread.contains("# my arawn config"));
        assert!(reread.contains("[integrations.atlassian]"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
    }

    #[test]
    fn load_doc_of_missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let doc = load_doc(&dir.path().join("nope.toml")).unwrap();
        assert!(doc.to_string().is_empty());
    }
}
