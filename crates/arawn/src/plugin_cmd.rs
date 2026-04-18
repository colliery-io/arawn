//! Plugin CLI commands — arawn plugin install/uninstall/enable/disable/list/marketplace.

use std::path::{Path, PathBuf};

use arawn_engine::plugins::{
    InstallScope, InstalledPluginsRegistry, MarketplaceSource,
    PluginIdentifier, add_marketplace, discover_plugins, install_plugin, list_marketplaces,
    uninstall_plugin,
};

/// Handle the `arawn plugin` subcommand.
pub fn run_plugin_command(args: &[String], plugins_root: &Path) -> Result<(), String> {
    if args.is_empty() {
        return print_plugin_help();
    }

    match args[0].as_str() {
        "install" | "i" => cmd_install(&args[1..], plugins_root),
        "uninstall" => cmd_uninstall(&args[1..], plugins_root),
        "enable" => cmd_enable(&args[1..], plugins_root),
        "disable" => cmd_disable(&args[1..], plugins_root),
        "list" | "ls" => cmd_list(plugins_root),
        "marketplace" => cmd_marketplace(&args[1..], plugins_root),
        "--help" | "-h" | "help" => print_plugin_help(),
        other => Err(format!("unknown plugin subcommand: '{other}'. Run `arawn plugin --help` for usage.")),
    }
}

fn cmd_install(args: &[String], plugins_root: &Path) -> Result<(), String> {
    let identifier_str = args
        .first()
        .ok_or("usage: arawn plugin install <name@marketplace>")?;

    let identifier = PluginIdentifier::parse(identifier_str).ok_or_else(|| {
        format!(
            "invalid plugin identifier: '{}'. Expected format: name@marketplace",
            identifier_str
        )
    })?;

    let scope = parse_scope(args)?;
    let cache_path = install_plugin(&identifier, scope, plugins_root, None)?;
    println!("Installed {} → {}", identifier, cache_path.display());
    Ok(())
}

fn cmd_uninstall(args: &[String], plugins_root: &Path) -> Result<(), String> {
    let identifier_str = args
        .first()
        .ok_or("usage: arawn plugin uninstall <name@marketplace>")?;

    let identifier = PluginIdentifier::parse(identifier_str).ok_or_else(|| {
        format!("invalid plugin identifier: '{}'", identifier_str)
    })?;

    let scope = parse_scope(args)?;
    uninstall_plugin(&identifier, scope, plugins_root, true)?;
    println!("Uninstalled {}", identifier);
    Ok(())
}

fn cmd_enable(args: &[String], plugins_root: &Path) -> Result<(), String> {
    let identifier_str = args
        .first()
        .ok_or("usage: arawn plugin enable <name@marketplace>")?;

    // Write to settings: enabledPlugins[id] = true
    update_enabled_plugins(plugins_root, identifier_str, true)?;
    println!("Enabled {}", identifier_str);
    Ok(())
}

fn cmd_disable(args: &[String], plugins_root: &Path) -> Result<(), String> {
    let identifier_str = args
        .first()
        .ok_or("usage: arawn plugin disable <name@marketplace>")?;

    update_enabled_plugins(plugins_root, identifier_str, false)?;
    println!("Disabled {}", identifier_str);
    Ok(())
}

fn cmd_list(plugins_root: &Path) -> Result<(), String> {
    let plugins = discover_plugins(plugins_root);
    let registry_path = plugins_root.join("installed_plugins.json");
    let registry = InstalledPluginsRegistry::load(&registry_path);

    if plugins.is_empty() && registry.plugins.is_empty() {
        println!("No plugins installed.");
        return Ok(());
    }

    println!("{:<40} {:<10} {:<10} SOURCE", "PLUGIN", "VERSION", "STATUS");
    println!("{}", "-".repeat(80));

    for plugin in &plugins {
        let version = plugin.manifest.version.as_deref().unwrap_or("-");
        let status = if plugin.enabled { "enabled" } else { "disabled" };
        let source = format!("{:?}", plugin.source);
        println!("{:<40} {:<10} {:<10} {}", plugin.id, version, status, source);
    }

    Ok(())
}

fn cmd_marketplace(args: &[String], plugins_root: &Path) -> Result<(), String> {
    if args.is_empty() {
        return Err("usage: arawn plugin marketplace <add|list>".into());
    }

    match args[0].as_str() {
        "add" => cmd_marketplace_add(&args[1..], plugins_root),
        "list" | "ls" => cmd_marketplace_list(plugins_root),
        other => Err(format!("unknown marketplace subcommand: '{other}'")),
    }
}

fn cmd_marketplace_add(args: &[String], plugins_root: &Path) -> Result<(), String> {
    let source_str = args
        .first()
        .ok_or("usage: arawn plugin marketplace add <github-repo-or-url>")?;

    // Parse the source — try github format (org/repo) first, then URL
    let (name, source) = parse_marketplace_source(source_str)?;

    let manifest = add_marketplace(&name, source, plugins_root)?;
    println!(
        "Added marketplace '{}' with {} plugins",
        manifest.name,
        manifest.plugins.len()
    );
    for plugin in &manifest.plugins {
        let version = plugin.version.as_deref().unwrap_or("-");
        let desc = plugin.description.as_deref().unwrap_or("");
        println!("  {} v{} — {}", plugin.name, version, desc);
    }
    Ok(())
}

fn cmd_marketplace_list(plugins_root: &Path) -> Result<(), String> {
    let marketplaces = list_marketplaces(plugins_root);

    if marketplaces.is_empty() {
        println!("No marketplaces registered.");
        return Ok(());
    }

    for (name, _entry, manifest) in &marketplaces {
        let plugin_count = manifest.as_ref().map_or(0, |m| m.plugins.len());
        println!("{}  ({} plugins)", name, plugin_count);
        if let Some(m) = manifest {
            for plugin in &m.plugins {
                let version = plugin.version.as_deref().unwrap_or("-");
                println!("  {} v{}", plugin.name, version);
            }
        }
    }

    Ok(())
}

/// Parse --scope flag from args. Defaults to User.
fn parse_scope(args: &[String]) -> Result<InstallScope, String> {
    for (i, arg) in args.iter().enumerate() {
        if arg == "--scope" {
            let scope_str = args.get(i + 1).ok_or("--scope requires a value")?;
            return match scope_str.as_str() {
                "user" => Ok(InstallScope::User),
                "project" => Ok(InstallScope::Project),
                other => Err(format!("invalid scope: '{other}'. Use 'user' or 'project'")),
            };
        }
    }
    Ok(InstallScope::User)
}

/// Parse a marketplace source string.
///
/// - `org/repo` → GitHub source
/// - `https://...` → Git URL source
/// - `/local/path` → Directory source
fn parse_marketplace_source(s: &str) -> Result<(String, MarketplaceSource), String> {
    if s.starts_with("https://") || s.starts_with("http://") || s.starts_with("git://") {
        // Git URL — derive name from last path segment
        let name = s
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("unknown")
            .trim_end_matches(".git")
            .to_string();
        Ok((
            name,
            MarketplaceSource::Git {
                url: s.to_string(),
                git_ref: None,
            },
        ))
    } else if s.starts_with('/') || s.starts_with('.') {
        // Local directory
        let path = PathBuf::from(s);
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("local")
            .to_string();
        Ok((
            name,
            MarketplaceSource::Directory {
                path: s.to_string(),
            },
        ))
    } else if s.contains('/') {
        // GitHub org/repo format
        let name = s.replace('/', "-");
        Ok((
            name,
            MarketplaceSource::GitHub {
                repo: s.to_string(),
                git_ref: None,
            },
        ))
    } else {
        Err(format!(
            "cannot parse marketplace source: '{}'. Use org/repo, https://url, or /local/path",
            s
        ))
    }
}

/// Update enabledPlugins in settings.json at the plugins root.
fn update_enabled_plugins(
    plugins_root: &Path,
    identifier: &str,
    enabled: bool,
) -> Result<(), String> {
    let settings_path = plugins_root
        .parent()
        .unwrap_or(plugins_root)
        .join("settings.json");

    // Read existing settings or create new
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = std::fs::read_to_string(&settings_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Update enabledPlugins
    let enabled_plugins = settings
        .as_object_mut()
        .ok_or("settings is not a JSON object")?
        .entry("enabledPlugins")
        .or_insert(serde_json::json!({}));

    enabled_plugins
        .as_object_mut()
        .ok_or("enabledPlugins is not a JSON object")?
        .insert(identifier.to_string(), serde_json::json!(enabled));

    // Write back
    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&settings_path, json).map_err(|e| e.to_string())
}

fn print_plugin_help() -> Result<(), String> {
    println!(
        "arawn plugin — manage plugins

USAGE:
    arawn plugin <command> [options]

COMMANDS:
    install <name@marketplace>     Install a plugin
    uninstall <name@marketplace>   Uninstall a plugin
    enable <name@marketplace>      Enable a disabled plugin
    disable <name@marketplace>     Disable without uninstalling
    list                           List installed plugins
    marketplace add <source>       Add a marketplace (org/repo, URL, or path)
    marketplace list               List registered marketplaces

OPTIONS:
    --scope <user|project>         Installation scope (default: user)

EXAMPLES:
    arawn plugin marketplace add colliery-io/metis
    arawn plugin install metis@colliery-io-metis
    arawn plugin list
    arawn plugin disable metis@colliery-io-metis"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_github_source() {
        let (name, source) = parse_marketplace_source("colliery-io/metis").unwrap();
        assert_eq!(name, "colliery-io-metis");
        assert!(matches!(source, MarketplaceSource::GitHub { repo, .. } if repo == "colliery-io/metis"));
    }

    #[test]
    fn parse_url_source() {
        let (name, source) =
            parse_marketplace_source("https://github.com/org/plugins.git").unwrap();
        assert_eq!(name, "plugins");
        assert!(matches!(source, MarketplaceSource::Git { .. }));
    }

    #[test]
    fn parse_directory_source() {
        let (name, source) = parse_marketplace_source("/tmp/my-plugins").unwrap();
        assert_eq!(name, "my-plugins");
        assert!(matches!(source, MarketplaceSource::Directory { .. }));
    }

    #[test]
    fn parse_relative_directory() {
        let (name, source) = parse_marketplace_source("./local-plugins").unwrap();
        assert_eq!(name, "local-plugins");
        assert!(matches!(source, MarketplaceSource::Directory { .. }));
    }

    #[test]
    fn parse_scope_default() {
        let args: Vec<String> = vec!["metis@market".into()];
        assert_eq!(parse_scope(&args).unwrap(), InstallScope::User);
    }

    #[test]
    fn parse_scope_project() {
        let args: Vec<String> = vec!["metis@market".into(), "--scope".into(), "project".into()];
        assert_eq!(parse_scope(&args).unwrap(), InstallScope::Project);
    }

    #[test]
    fn parse_scope_invalid() {
        let args: Vec<String> = vec!["metis@market".into(), "--scope".into(), "invalid".into()];
        assert!(parse_scope(&args).is_err());
    }
}
