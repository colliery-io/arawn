//! Plugin installation — install/uninstall plugins into the versioned cache
//! and track installations in installed_plugins.json.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::info;

use super::loader::PluginIdentifier;
use super::marketplace::{
    KnownMarketplaces, MarketplacePlugin, PluginSourceRef, fetch_marketplace, resolve_plugin,
};

/// Installation scope — where the enablement is recorded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallScope {
    User,
    Project,
}

/// A single installation record for a plugin at a specific scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallRecord {
    pub scope: InstallScope,
    pub install_path: String,
    pub version: String,
    pub installed_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
}

/// The installed_plugins.json registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPluginsRegistry {
    pub version: u32,
    pub plugins: HashMap<String, Vec<InstallRecord>>,
}

impl Default for InstalledPluginsRegistry {
    fn default() -> Self {
        Self {
            version: 2,
            plugins: HashMap::new(),
        }
    }
}

impl InstalledPluginsRegistry {
    /// Load from a JSON file. Returns empty if missing or invalid.
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

    /// Add an installation record. Replaces existing record for same scope.
    pub fn add(&mut self, id: &str, record: InstallRecord) {
        let records = self.plugins.entry(id.to_string()).or_default();
        records.retain(|r| r.scope != record.scope);
        records.push(record);
    }

    /// Remove all records for a plugin at a specific scope.
    /// Returns true if the plugin has no remaining records (fully uninstalled).
    pub fn remove(&mut self, id: &str, scope: &InstallScope) -> bool {
        if let Some(records) = self.plugins.get_mut(id) {
            records.retain(|r| &r.scope != scope);
            if records.is_empty() {
                self.plugins.remove(id);
                return true;
            }
        }
        false
    }

    /// Get records for a plugin.
    pub fn get(&self, id: &str) -> Option<&Vec<InstallRecord>> {
        self.plugins.get(id)
    }
}

/// Install a plugin from a marketplace into the versioned cache.
///
/// Steps:
/// 1. Look up marketplace in known_marketplaces.json
/// 2. Fetch marketplace manifest, find the plugin
/// 3. Clone plugin source into cache/{marketplace}/{plugin}/{version}/
/// 4. Register in installed_plugins.json
pub fn install_plugin(
    identifier: &PluginIdentifier,
    scope: InstallScope,
    plugins_root: &Path,
    project_path: Option<&Path>,
) -> Result<PathBuf, String> {
    let known_path = plugins_root.join("known_marketplaces.json");
    let known = KnownMarketplaces::load(&known_path);

    let market_entry = known
        .get(&identifier.marketplace)
        .ok_or_else(|| {
            format!(
                "marketplace '{}' not found. Add it first with `arawn plugin marketplace add`",
                identifier.marketplace
            )
        })?;

    let marketplaces_dir = plugins_root.join("marketplaces");
    let manifest =
        fetch_marketplace(&market_entry.source, &identifier.marketplace, &marketplaces_dir)?;

    let plugin_entry = resolve_plugin(&manifest, &identifier.name).ok_or_else(|| {
        format!(
            "plugin '{}' not found in marketplace '{}'",
            identifier.name, identifier.marketplace
        )
    })?;

    let version = plugin_entry
        .version
        .as_deref()
        .unwrap_or("latest")
        .to_string();

    let cache_path = plugins_root
        .join("cache")
        .join(&identifier.marketplace)
        .join(&identifier.name)
        .join(&version);

    // Resolve the marketplace directory (for relative path sources)
    let marketplace_clone_dir = market_entry
        .install_location
        .as_ref()
        .map(|loc| PathBuf::from(loc));

    // Clone/download the plugin into the cache
    clone_plugin_to_cache(
        plugin_entry,
        &market_entry.source,
        &cache_path,
        marketplace_clone_dir.as_deref(),
    )?;

    // Register in installed_plugins.json
    let registry_path = plugins_root.join("installed_plugins.json");
    let mut registry = InstalledPluginsRegistry::load(&registry_path);
    registry.add(
        &identifier.to_string(),
        InstallRecord {
            scope,
            install_path: cache_path.to_string_lossy().to_string(),
            version: version.clone(),
            installed_at: chrono::Utc::now().to_rfc3339(),
            project_path: project_path.map(|p| p.to_string_lossy().to_string()),
        },
    );
    registry.save(&registry_path)?;

    info!(
        plugin = %identifier,
        version,
        cache = ?cache_path,
        "plugin installed"
    );

    Ok(cache_path)
}

/// Uninstall a plugin — remove from registry, optionally remove cache.
pub fn uninstall_plugin(
    identifier: &PluginIdentifier,
    scope: InstallScope,
    plugins_root: &Path,
    remove_cache: bool,
) -> Result<(), String> {
    let registry_path = plugins_root.join("installed_plugins.json");
    let mut registry = InstalledPluginsRegistry::load(&registry_path);

    let fully_removed = registry.remove(&identifier.to_string(), &scope);
    registry.save(&registry_path)?;

    if fully_removed && remove_cache {
        let cache_dir = plugins_root
            .join("cache")
            .join(&identifier.marketplace)
            .join(&identifier.name);
        if cache_dir.exists() {
            std::fs::remove_dir_all(&cache_dir).map_err(|e| {
                format!("failed to remove cache dir {}: {e}", cache_dir.display())
            })?;
        }
    }

    info!(plugin = %identifier, "plugin uninstalled");
    Ok(())
}

/// Clone a plugin's source into the cache directory.
fn clone_plugin_to_cache(
    plugin: &MarketplacePlugin,
    market_source: &super::marketplace::MarketplaceSource,
    cache_path: &Path,
    marketplace_dir: Option<&Path>,
) -> Result<(), String> {
    // If cache already exists, treat as update (remove and re-clone)
    if cache_path.exists() {
        std::fs::remove_dir_all(cache_path)
            .map_err(|e| format!("failed to clear cache: {e}"))?;
    }

    // Check for relative path source — copy from marketplace clone dir
    if let Some(ref src) = plugin.source {
        if let Some(rel_path) = src.relative_path() {
            let market_dir = marketplace_dir.ok_or_else(|| {
                "plugin source is a relative path but no marketplace directory available".to_string()
            })?;
            let stripped = rel_path.strip_prefix("./").unwrap_or(rel_path);
            let source_dir = market_dir.join(stripped);
            if !source_dir.exists() {
                return Err(format!(
                    "plugin source path '{}' not found in marketplace at {}",
                    rel_path,
                    market_dir.display()
                ));
            }
            std::fs::create_dir_all(cache_path)
                .map_err(|e| format!("failed to create cache dir: {e}"))?;
            copy_dir_recursive(&source_dir, cache_path)?;
            return Ok(());
        }
    }

    // Determine git URL and optional subdirectory
    let (git_url, git_ref, subdir) = if let Some(ref src) = plugin.source {
        match src {
            PluginSourceRef::GitHub {
                repo,
                git_ref,
                path,
                ..
            } => (
                format!("https://github.com/{}.git", repo),
                git_ref.clone(),
                path.clone(),
            ),
            PluginSourceRef::Git {
                url,
                git_ref,
                path,
                ..
            } => (url.clone(), git_ref.clone(), path.clone()),
            PluginSourceRef::RelativePath(_) => unreachable!("handled above"),
        }
    } else {
        // Fall back to marketplace source
        let url = market_source
            .git_url()
            .ok_or_else(|| "no git URL available for plugin source".to_string())?;
        (
            url,
            market_source.git_ref().map(String::from),
            None,
        )
    };

    // Clone to temp directory
    let temp_dir = tempfile::tempdir().map_err(|e| format!("failed to create temp dir: {e}"))?;
    let temp_clone = temp_dir.path().join("clone");

    let mut cmd = std::process::Command::new("git");
    cmd.arg("clone").arg("--depth").arg("1");
    if let Some(ref r) = git_ref {
        cmd.arg("--branch").arg(r);
    }
    cmd.arg(&git_url).arg(&temp_clone);

    let output = cmd
        .output()
        .map_err(|e| format!("failed to run git clone: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git clone failed: {stderr}"));
    }

    // If subdirectory specified (monorepo), copy only that subdirectory
    let source_dir = if let Some(ref sub) = subdir {
        let sub_path = temp_clone.join(sub);
        if !sub_path.exists() {
            return Err(format!(
                "subdirectory '{}' not found in cloned repo",
                sub
            ));
        }
        sub_path
    } else {
        temp_clone.clone()
    };

    // Copy to cache path
    std::fs::create_dir_all(cache_path)
        .map_err(|e| format!("failed to create cache dir: {e}"))?;

    copy_dir_recursive(&source_dir, cache_path)?;

    Ok(())
}

/// Recursively copy a directory's contents.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            // Skip .git directory
            if entry.file_name() == ".git" {
                continue;
            }
            std::fs::create_dir_all(&dst_path).map_err(|e| e.to_string())?;
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn registry_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("installed_plugins.json");

        let mut registry = InstalledPluginsRegistry::default();
        registry.add(
            "metis@colliery-io-metis",
            InstallRecord {
                scope: InstallScope::User,
                install_path: "/tmp/cache/metis/2.0.4".into(),
                version: "2.0.4".into(),
                installed_at: "2026-04-04T12:00:00Z".into(),
                project_path: None,
            },
        );

        registry.save(&path).unwrap();

        let loaded = InstalledPluginsRegistry::load(&path);
        assert_eq!(loaded.version, 2);
        let records = loaded.get("metis@colliery-io-metis").unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].version, "2.0.4");
        assert_eq!(records[0].scope, InstallScope::User);
    }

    #[test]
    fn registry_replace_same_scope() {
        let mut registry = InstalledPluginsRegistry::default();
        registry.add(
            "test@market",
            InstallRecord {
                scope: InstallScope::User,
                install_path: "/v1".into(),
                version: "1.0".into(),
                installed_at: "".into(),
                project_path: None,
            },
        );
        registry.add(
            "test@market",
            InstallRecord {
                scope: InstallScope::User,
                install_path: "/v2".into(),
                version: "2.0".into(),
                installed_at: "".into(),
                project_path: None,
            },
        );

        let records = registry.get("test@market").unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].version, "2.0");
    }

    #[test]
    fn registry_multiple_scopes() {
        let mut registry = InstalledPluginsRegistry::default();
        registry.add(
            "test@market",
            InstallRecord {
                scope: InstallScope::User,
                install_path: "/user".into(),
                version: "1.0".into(),
                installed_at: "".into(),
                project_path: None,
            },
        );
        registry.add(
            "test@market",
            InstallRecord {
                scope: InstallScope::Project,
                install_path: "/project".into(),
                version: "1.0".into(),
                installed_at: "".into(),
                project_path: Some("/my/project".into()),
            },
        );

        let records = registry.get("test@market").unwrap();
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn registry_remove_one_scope() {
        let mut registry = InstalledPluginsRegistry::default();
        registry.add(
            "test@market",
            InstallRecord {
                scope: InstallScope::User,
                install_path: "/user".into(),
                version: "1.0".into(),
                installed_at: "".into(),
                project_path: None,
            },
        );
        registry.add(
            "test@market",
            InstallRecord {
                scope: InstallScope::Project,
                install_path: "/project".into(),
                version: "1.0".into(),
                installed_at: "".into(),
                project_path: None,
            },
        );

        let fully_removed = registry.remove("test@market", &InstallScope::User);
        assert!(!fully_removed); // project scope still exists
        assert_eq!(registry.get("test@market").unwrap().len(), 1);
    }

    #[test]
    fn registry_remove_last_scope() {
        let mut registry = InstalledPluginsRegistry::default();
        registry.add(
            "test@market",
            InstallRecord {
                scope: InstallScope::User,
                install_path: "/user".into(),
                version: "1.0".into(),
                installed_at: "".into(),
                project_path: None,
            },
        );

        let fully_removed = registry.remove("test@market", &InstallScope::User);
        assert!(fully_removed);
        assert!(registry.get("test@market").is_none());
    }

    #[test]
    fn registry_load_missing() {
        let registry = InstalledPluginsRegistry::load(Path::new("/nonexistent.json"));
        assert_eq!(registry.version, 2);
        assert!(registry.plugins.is_empty());
    }

    #[test]
    fn copy_dir_skips_git() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();

        std::fs::write(src.path().join("file.txt"), "hello").unwrap();
        std::fs::create_dir(src.path().join(".git")).unwrap();
        std::fs::write(src.path().join(".git/HEAD"), "ref").unwrap();
        std::fs::create_dir(src.path().join("subdir")).unwrap();
        std::fs::write(src.path().join("subdir/nested.txt"), "world").unwrap();

        let target = dst.path().join("output");
        std::fs::create_dir(&target).unwrap();
        copy_dir_recursive(src.path(), &target).unwrap();

        assert!(target.join("file.txt").exists());
        assert!(target.join("subdir/nested.txt").exists());
        assert!(!target.join(".git").exists()); // skipped
    }
}
