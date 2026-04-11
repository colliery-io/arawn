//! Integration tests: plugin discovery, manifest parsing, and component loading.

use std::sync::Arc;

use arawn_engine::plugins::{
    PluginComponents, discover_plugins, load_plugin_components, load_plugin_dir,
    register_plugin_skills,
};
use arawn_engine::skills::SkillRegistry;
use tempfile::TempDir;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Create a minimal valid plugin directory with plugin.json.
fn write_plugin_json(dir: &std::path::Path, name: &str) {
    let manifest = serde_json::json!({
        "name": name,
        "version": "1.0.0",
        "description": format!("Test plugin: {name}")
    });
    std::fs::write(
        dir.join("plugin.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
}

/// Create a plugin cache directory: cache/{marketplace}/{plugin}/{version}/
fn create_cache_plugin(root: &std::path::Path, marketplace: &str, name: &str) -> std::path::PathBuf {
    let version_dir = root
        .join("cache")
        .join(marketplace)
        .join(name)
        .join("1.0.0");
    std::fs::create_dir_all(&version_dir).unwrap();
    write_plugin_json(&version_dir, name);
    version_dir
}

/// Write a skill markdown file into a directory.
fn write_skill(dir: &std::path::Path, filename: &str, description: &str, prompt: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join(filename),
        format!(
            "---\ndescription: \"{description}\"\nuser_invocable: true\n---\n\n{prompt}\n"
        ),
    )
    .unwrap();
}

/// Write an agent markdown file into a directory.
fn write_agent(dir: &std::path::Path, filename: &str, name: &str, description: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join(filename),
        format!(
            "---\nname: {name}\ndescription: \"{description}\"\n---\n\nAgent instructions here.\n"
        ),
    )
    .unwrap();
}

/// Write a hooks.json file.
fn write_hooks_json(dir: &std::path::Path) {
    let hooks_dir = dir.join("hooks");
    std::fs::create_dir_all(&hooks_dir).unwrap();
    std::fs::write(
        hooks_dir.join("hooks.json"),
        r#"{"PreToolUse": [{"matcher": "shell", "hooks": [{"type": "command", "command": "exit 0"}]}]}"#,
    )
    .unwrap();
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[test]
fn discover_plugins_finds_cache_plugin() {
    let tmp = TempDir::new().unwrap();
    create_cache_plugin(tmp.path(), "test-market", "my-plugin");

    let plugins = discover_plugins(tmp.path());
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].manifest.name, "my-plugin");
    assert_eq!(plugins[0].id.marketplace, "test-market");
}

#[test]
fn discover_plugins_finds_multiple() {
    let tmp = TempDir::new().unwrap();
    create_cache_plugin(tmp.path(), "market-a", "plugin-one");
    create_cache_plugin(tmp.path(), "market-a", "plugin-two");
    create_cache_plugin(tmp.path(), "market-b", "plugin-three");

    let plugins = discover_plugins(tmp.path());
    assert_eq!(plugins.len(), 3);
}

#[test]
fn load_plugin_dir_parses_manifest() {
    let tmp = TempDir::new().unwrap();
    write_plugin_json(tmp.path(), "test-plugin");

    let plugin = load_plugin_dir(tmp.path()).expect("should load plugin");
    assert_eq!(plugin.manifest.name, "test-plugin");
    assert!(plugin.enabled);
}

#[test]
fn load_plugin_components_loads_skills() {
    let tmp = TempDir::new().unwrap();
    let plugin_dir = create_cache_plugin(tmp.path(), "test-market", "skill-plugin");

    // Create skills directory by convention (auto-discovered)
    write_skill(
        &plugin_dir.join("skills"),
        "greeting.md",
        "Greet the user",
        "Say hello warmly.",
    );

    let plugins = discover_plugins(tmp.path());
    assert_eq!(plugins.len(), 1);

    let components = load_plugin_components(&plugins[0]);
    assert_eq!(
        components.skills.len(),
        1,
        "should have loaded 1 skill, got: {:?}",
        components.errors
    );
    // Skills are namespaced: "plugin_name:skill_name"
    assert!(
        components.skills[0].name.contains("skill-plugin:"),
        "skill should be namespaced, got: {}",
        components.skills[0].name
    );
}

#[test]
fn load_plugin_components_loads_agents() {
    let tmp = TempDir::new().unwrap();
    let plugin_dir = create_cache_plugin(tmp.path(), "test-market", "agent-plugin");

    write_agent(
        &plugin_dir.join("agents"),
        "helper.md",
        "helper",
        "A helper agent",
    );

    let plugins = discover_plugins(tmp.path());
    let components = load_plugin_components(&plugins[0]);
    assert_eq!(
        components.agents.len(),
        1,
        "should have loaded 1 agent, errors: {:?}",
        components.errors
    );
    assert!(
        components.agents[0].name.contains("agent-plugin:"),
        "agent should be namespaced, got: {}",
        components.agents[0].name
    );
}

#[test]
fn load_plugin_components_loads_hooks() {
    let tmp = TempDir::new().unwrap();
    let version_dir = tmp
        .path()
        .join("cache")
        .join("test-market")
        .join("hook-plugin")
        .join("1.0.0");
    std::fs::create_dir_all(&version_dir).unwrap();

    // Write hooks file (wrapped in settings format: {"hooks": {...}})
    std::fs::write(
        version_dir.join("hooks.json"),
        r#"{"hooks": {"PreToolUse": [{"matcher": "shell", "hooks": [{"type": "command", "command": "exit 0"}]}]}}"#,
    )
    .unwrap();

    // Manifest must declare hooks path for component loader to read it
    let manifest = serde_json::json!({
        "name": "hook-plugin",
        "hooks": "./hooks.json"
    });
    std::fs::write(
        version_dir.join("plugin.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let plugins = discover_plugins(tmp.path());
    assert_eq!(plugins.len(), 1);

    let components = load_plugin_components(&plugins[0]);
    assert!(
        components.hooks.is_some(),
        "should have loaded hooks, errors: {:?}",
        components.errors
    );
    let hooks = components.hooks.unwrap();
    assert!(
        !hooks.is_empty(),
        "hooks config should not be empty"
    );
}

#[test]
fn register_plugin_skills_namespaces_into_registry() {
    let tmp = TempDir::new().unwrap();
    let plugin_dir = create_cache_plugin(tmp.path(), "test-market", "my-plugin");

    write_skill(
        &plugin_dir.join("skills"),
        "deploy.md",
        "Deploy app",
        "Deploy the application.",
    );

    let plugins = discover_plugins(tmp.path());
    let components = load_plugin_components(&plugins[0]);

    let registry = Arc::new(SkillRegistry::new());
    register_plugin_skills(&registry, components.skills);

    // Should be accessible by namespaced name (registry also has built-in skills)
    let skill = registry.get("my-plugin:deploy").expect("should find namespaced skill");
    assert!(skill.prompt.contains("Deploy the application"));
}

#[test]
fn invalid_manifest_gracefully_skipped() {
    let tmp = TempDir::new().unwrap();

    // Valid plugin
    create_cache_plugin(tmp.path(), "test-market", "good-plugin");

    // Invalid plugin — broken JSON
    let bad_dir = tmp
        .path()
        .join("cache")
        .join("test-market")
        .join("bad-plugin");
    std::fs::create_dir_all(&bad_dir).unwrap();
    std::fs::write(bad_dir.join("plugin.json"), "{ invalid json }}}").unwrap();

    let plugins = discover_plugins(tmp.path());
    // Only the valid plugin should load
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].manifest.name, "good-plugin");
}

#[test]
fn plugin_with_mixed_valid_invalid_components() {
    let tmp = TempDir::new().unwrap();
    let plugin_dir = create_cache_plugin(tmp.path(), "test-market", "mixed-plugin");

    // Valid skill
    write_skill(
        &plugin_dir.join("skills"),
        "valid.md",
        "A valid skill",
        "Valid skill content.",
    );

    // Invalid skill — broken frontmatter
    std::fs::write(
        plugin_dir.join("skills").join("broken.md"),
        "---\ninvalid: [yaml: broken\n---\nContent.\n",
    )
    .unwrap();

    let plugins = discover_plugins(tmp.path());
    let components = load_plugin_components(&plugins[0]);

    // Valid skill should load
    assert!(
        !components.skills.is_empty(),
        "at least 1 valid skill should load"
    );
    // The valid one should have the right name
    assert!(components
        .skills
        .iter()
        .any(|s| s.name.contains("valid")));
}

#[test]
fn empty_cache_returns_no_plugins() {
    let tmp = TempDir::new().unwrap();
    // No cache directory at all
    let plugins = discover_plugins(tmp.path());
    assert!(plugins.is_empty());
}
