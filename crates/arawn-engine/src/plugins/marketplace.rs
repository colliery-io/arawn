//! Marketplace system — fetch and parse marketplace manifests to discover plugins.
//!
//! Marketplaces are git repositories (or local directories) containing a
//! `marketplace.json` (or `.claude-plugin/marketplace.json`) that lists
//! available plugins with their sources and versions.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Source type for a marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "source")]
pub enum MarketplaceSource {
    /// GitHub repository: `{ "source": "github", "repo": "owner/repo" }`
    #[serde(rename = "github")]
    GitHub {
        repo: String,
        #[serde(rename = "ref", default)]
        git_ref: Option<String>,
    },
    /// Arbitrary git URL: `{ "source": "git", "url": "https://..." }`
    #[serde(rename = "git")]
    Git {
        url: String,
        #[serde(rename = "ref", default)]
        git_ref: Option<String>,
    },
    /// Local directory: `{ "source": "directory", "path": "/local/path" }`
    #[serde(rename = "directory")]
    Directory { path: String },
}

impl MarketplaceSource {
    /// Get the git clone URL for this source.
    pub fn git_url(&self) -> Option<String> {
        match self {
            MarketplaceSource::GitHub { repo, .. } => {
                Some(format!("https://github.com/{}.git", repo))
            }
            MarketplaceSource::Git { url, .. } => Some(url.clone()),
            MarketplaceSource::Directory { .. } => None,
        }
    }

    /// Get the git ref (branch/tag) to checkout.
    pub fn git_ref(&self) -> Option<&str> {
        match self {
            MarketplaceSource::GitHub { git_ref, .. }
            | MarketplaceSource::Git { git_ref, .. } => git_ref.as_deref(),
            MarketplaceSource::Directory { .. } => None,
        }
    }
}

/// A marketplace manifest (marketplace.json) — lists available plugins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceManifest {
    /// Marketplace name.
    pub name: String,
    /// Available plugins.
    #[serde(default)]
    pub plugins: Vec<MarketplacePlugin>,
    /// Optional metadata.
    #[serde(default)]
    pub metadata: Option<MarketplaceMetadata>,
}

/// A plugin entry in a marketplace manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplacePlugin {
    /// Plugin name.
    pub name: String,
    /// Plugin version.
    #[serde(default)]
    pub version: Option<String>,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
    /// Source for fetching the plugin content.
    /// Can be a simple string (relative path within marketplace repo)
    /// or a structured source reference.
    #[serde(default, deserialize_with = "deserialize_plugin_source")]
    pub source: Option<PluginSourceRef>,
}

/// Reference to a plugin's source within a marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginSourceRef {
    /// Simple relative path within the marketplace repo (e.g. "./plugins/metis").
    RelativePath(String),
    /// GitHub repo with optional subdirectory.
    GitHub {
        #[serde(rename = "source")]
        source_type: GithubSourceTag,
        repo: String,
        #[serde(rename = "ref", default)]
        git_ref: Option<String>,
        #[serde(default)]
        path: Option<String>,
    },
    /// Git URL with optional subdirectory.
    Git {
        #[serde(rename = "source")]
        source_type: GitSourceTag,
        url: String,
        #[serde(rename = "ref", default)]
        git_ref: Option<String>,
        #[serde(default)]
        path: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GithubSourceTag {
    Github,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GitSourceTag {
    Git,
}

impl PluginSourceRef {
    /// Get the relative path within the marketplace repo, if this is a relative path source.
    pub fn relative_path(&self) -> Option<&str> {
        match self {
            PluginSourceRef::RelativePath(p) => Some(p),
            _ => None,
        }
    }
}

fn deserialize_plugin_source<'de, D>(deserializer: D) -> Result<Option<PluginSourceRef>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(serde_json::Value::String(s)) => Ok(Some(PluginSourceRef::RelativePath(s))),
        Some(obj) => {
            let src: PluginSourceRef =
                serde_json::from_value(obj).map_err(serde::de::Error::custom)?;
            Ok(Some(src))
        }
    }
}

/// Marketplace metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceMetadata {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Entry in known_marketplaces.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceEntry {
    pub source: MarketplaceSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install_location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
}

/// Known marketplaces registry — read/write `known_marketplaces.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KnownMarketplaces {
    #[serde(flatten)]
    pub entries: HashMap<String, MarketplaceEntry>,
}

impl KnownMarketplaces {
    /// Load from a JSON file. Returns empty if file missing or invalid.
    pub fn load(path: &Path) -> Self {
        if !path.exists() {
            return Self::default();
        }
        match std::fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save to a JSON file.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }

    /// Add or update a marketplace entry.
    pub fn add(&mut self, name: String, entry: MarketplaceEntry) {
        self.entries.insert(name, entry);
    }

    /// Get a marketplace entry by name.
    pub fn get(&self, name: &str) -> Option<&MarketplaceEntry> {
        self.entries.get(name)
    }

    /// List all marketplace names.
    pub fn names(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }
}

/// Fetch a marketplace manifest by cloning/pulling a git repo.
///
/// Clones the repo into `{marketplaces_dir}/{name}/` and reads the manifest.
pub fn fetch_marketplace(
    source: &MarketplaceSource,
    name: &str,
    marketplaces_dir: &Path,
) -> Result<MarketplaceManifest, String> {
    match source {
        MarketplaceSource::Directory { path } => {
            let dir = PathBuf::from(path);
            read_marketplace_manifest(&dir)
        }
        _ => {
            let url = source
                .git_url()
                .ok_or_else(|| "no git URL for source".to_string())?;
            let clone_dir = marketplaces_dir.join(name);

            if clone_dir.exists() {
                // Pull latest
                git_pull(&clone_dir, source.git_ref())?;
            } else {
                // Clone
                git_clone(&url, &clone_dir, source.git_ref())?;
            }

            read_marketplace_manifest(&clone_dir)
        }
    }
}

/// Add a marketplace source: fetch it and register in known_marketplaces.json.
pub fn add_marketplace(
    name: &str,
    source: MarketplaceSource,
    plugins_root: &Path,
) -> Result<MarketplaceManifest, String> {
    let marketplaces_dir = plugins_root.join("marketplaces");
    let manifest = fetch_marketplace(&source, name, &marketplaces_dir)?;

    // Register in known_marketplaces.json
    let known_path = plugins_root.join("known_marketplaces.json");
    let mut known = KnownMarketplaces::load(&known_path);
    known.add(
        name.to_string(),
        MarketplaceEntry {
            source,
            install_location: Some(
                marketplaces_dir
                    .join(name)
                    .to_string_lossy()
                    .to_string(),
            ),
            last_updated: Some(chrono::Utc::now().to_rfc3339()),
        },
    );
    known.save(&known_path)?;

    info!(marketplace = name, plugins = manifest.plugins.len(), "added marketplace");
    Ok(manifest)
}

/// List all marketplaces and their available plugins.
pub fn list_marketplaces(
    plugins_root: &Path,
) -> Vec<(String, MarketplaceEntry, Option<MarketplaceManifest>)> {
    let known_path = plugins_root.join("known_marketplaces.json");
    let known = KnownMarketplaces::load(&known_path);
    let marketplaces_dir = plugins_root.join("marketplaces");

    known
        .entries
        .into_iter()
        .map(|(name, entry)| {
            let manifest = fetch_marketplace(&entry.source, &name, &marketplaces_dir).ok();
            (name, entry, manifest)
        })
        .collect()
}

/// Find a plugin entry in a marketplace manifest by name.
pub fn resolve_plugin<'a>(
    manifest: &'a MarketplaceManifest,
    plugin_name: &str,
) -> Option<&'a MarketplacePlugin> {
    manifest.plugins.iter().find(|p| p.name == plugin_name)
}

/// Read a marketplace manifest from a directory.
///
/// Checks `.claude-plugin/marketplace.json` first, then `marketplace.json` at root.
fn read_marketplace_manifest(dir: &Path) -> Result<MarketplaceManifest, String> {
    let claude_path = dir.join(".claude-plugin").join("marketplace.json");
    let root_path = dir.join("marketplace.json");

    let manifest_path = if claude_path.exists() {
        claude_path
    } else if root_path.exists() {
        root_path
    } else {
        return Err(format!(
            "no marketplace.json found in {} (checked .claude-plugin/ and root)",
            dir.display()
        ));
    };

    let content = std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| format!("invalid marketplace.json: {e}"))
}

/// Clone a git repo to a directory.
fn git_clone(url: &str, target: &Path, git_ref: Option<&str>) -> Result<(), String> {
    let mut cmd = std::process::Command::new("git");
    cmd.arg("clone").arg("--depth").arg("1");
    if let Some(r) = git_ref {
        cmd.arg("--branch").arg(r);
    }
    cmd.arg(url).arg(target);

    let output = cmd
        .output()
        .map_err(|e| format!("failed to run git clone: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git clone failed: {stderr}"));
    }

    Ok(())
}

/// Pull latest changes in an existing clone.
fn git_pull(dir: &Path, git_ref: Option<&str>) -> Result<(), String> {
    if let Some(r) = git_ref {
        let output = std::process::Command::new("git")
            .args(["checkout", r])
            .current_dir(dir)
            .output()
            .map_err(|e| format!("git checkout failed: {e}"))?;
        if !output.status.success() {
            warn!(dir = ?dir, ref_ = r, "git checkout failed, continuing with current state");
        }
    }

    let output = std::process::Command::new("git")
        .args(["pull", "--ff-only"])
        .current_dir(dir)
        .output()
        .map_err(|e| format!("git pull failed: {e}"))?;

    if !output.status.success() {
        // Non-fatal — use existing state
        warn!(dir = ?dir, "git pull failed, using cached state");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_marketplace(dir: &Path, json: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("marketplace.json"), json).unwrap();
    }

    fn sample_manifest_json() -> &'static str {
        r#"{
            "name": "test-market",
            "plugins": [
                {
                    "name": "plugin-a",
                    "version": "1.0.0",
                    "description": "Plugin A"
                },
                {
                    "name": "plugin-b",
                    "version": "2.0.0",
                    "description": "Plugin B",
                    "source": {
                        "source": "github",
                        "repo": "org/plugin-b",
                        "path": "plugins/b"
                    }
                }
            ]
        }"#
    }

    #[test]
    fn parse_marketplace_manifest() {
        let manifest: MarketplaceManifest =
            serde_json::from_str(sample_manifest_json()).unwrap();
        assert_eq!(manifest.name, "test-market");
        assert_eq!(manifest.plugins.len(), 2);
        assert_eq!(manifest.plugins[0].name, "plugin-a");
        assert_eq!(manifest.plugins[1].name, "plugin-b");
        assert!(manifest.plugins[1].source.is_some());
    }

    #[test]
    fn read_manifest_from_root() {
        let dir = TempDir::new().unwrap();
        write_marketplace(dir.path(), sample_manifest_json());

        let manifest = read_marketplace_manifest(dir.path()).unwrap();
        assert_eq!(manifest.name, "test-market");
    }

    #[test]
    fn read_manifest_from_claude_plugin_dir() {
        let dir = TempDir::new().unwrap();
        let claude_dir = dir.path().join(".claude-plugin");
        write_marketplace(&claude_dir, sample_manifest_json());

        let manifest = read_marketplace_manifest(dir.path()).unwrap();
        assert_eq!(manifest.name, "test-market");
    }

    #[test]
    fn read_manifest_missing() {
        let dir = TempDir::new().unwrap();
        let result = read_marketplace_manifest(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn resolve_plugin_found() {
        let manifest: MarketplaceManifest =
            serde_json::from_str(sample_manifest_json()).unwrap();
        let plugin = resolve_plugin(&manifest, "plugin-b").unwrap();
        assert_eq!(plugin.name, "plugin-b");
        assert_eq!(plugin.version.as_deref(), Some("2.0.0"));
    }

    #[test]
    fn resolve_plugin_not_found() {
        let manifest: MarketplaceManifest =
            serde_json::from_str(sample_manifest_json()).unwrap();
        assert!(resolve_plugin(&manifest, "nonexistent").is_none());
    }

    #[test]
    fn fetch_from_directory_source() {
        let dir = TempDir::new().unwrap();
        let market_dir = dir.path().join("my-market");
        write_marketplace(&market_dir, sample_manifest_json());

        let source = MarketplaceSource::Directory {
            path: market_dir.to_string_lossy().to_string(),
        };
        let manifest = fetch_marketplace(&source, "my-market", dir.path()).unwrap();
        assert_eq!(manifest.name, "test-market");
    }

    #[test]
    fn known_marketplaces_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("known_marketplaces.json");

        let mut known = KnownMarketplaces::default();
        known.add(
            "test-market".into(),
            MarketplaceEntry {
                source: MarketplaceSource::GitHub {
                    repo: "org/plugins".into(),
                    git_ref: None,
                },
                install_location: Some("/tmp/test".into()),
                last_updated: Some("2026-04-04T12:00:00Z".into()),
            },
        );

        known.save(&path).unwrap();

        let loaded = KnownMarketplaces::load(&path);
        assert_eq!(loaded.entries.len(), 1);
        assert!(loaded.get("test-market").is_some());
    }

    #[test]
    fn known_marketplaces_missing_file() {
        let known = KnownMarketplaces::load(Path::new("/nonexistent/known.json"));
        assert!(known.entries.is_empty());
    }

    #[test]
    fn marketplace_source_git_url() {
        let gh = MarketplaceSource::GitHub {
            repo: "org/repo".into(),
            git_ref: None,
        };
        assert_eq!(
            gh.git_url(),
            Some("https://github.com/org/repo.git".to_string())
        );

        let git = MarketplaceSource::Git {
            url: "https://example.com/repo.git".into(),
            git_ref: Some("v1.0".into()),
        };
        assert_eq!(
            git.git_url(),
            Some("https://example.com/repo.git".to_string())
        );
        assert_eq!(git.git_ref(), Some("v1.0"));

        let dir = MarketplaceSource::Directory {
            path: "/local".into(),
        };
        assert!(dir.git_url().is_none());
    }

    #[test]
    fn plugin_source_ref_deserialization() {
        let json = r#"{
            "source": "github",
            "repo": "org/plugin",
            "ref": "main",
            "path": "plugins/my-plugin"
        }"#;
        let src: PluginSourceRef = serde_json::from_str(json).unwrap();
        match src {
            PluginSourceRef::GitHub { repo, git_ref, path, .. } => {
                assert_eq!(repo, "org/plugin");
                assert_eq!(git_ref.as_deref(), Some("main"));
                assert_eq!(path.as_deref(), Some("plugins/my-plugin"));
            }
            _ => panic!("expected GitHub source"),
        }
    }
}
