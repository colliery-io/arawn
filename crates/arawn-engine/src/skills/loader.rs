use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

use tracing::{info, warn};

use super::definition::{SkillDefinition, SkillSource, parse_skill_markdown};

/// Registry of loaded skills, queryable by name.
pub struct SkillRegistry {
    skills: RwLock<HashMap<String, SkillDefinition>>,
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillRegistry {
    pub fn new() -> Self {
        let registry = Self {
            skills: RwLock::new(HashMap::new()),
        };
        registry.register_builtins();
        registry
    }

    /// Register built-in skills that ship with the arawn binary.
    fn register_builtins(&self) {
        let builtins: &[(&str, &str)] = &[
            ("workflows", include_str!("builtin/workflows.md")),
        ];

        for (default_name, content) in builtins {
            match parse_skill_markdown(content, default_name) {
                Ok(mut skill) => {
                    skill.source = SkillSource::BuiltIn;
                    self.register(skill);
                }
                Err(e) => {
                    warn!(name = default_name, error = %e, "failed to parse built-in skill");
                }
            }
        }
    }

    /// Register a skill. If a skill with the same name exists, it's replaced.
    pub fn register(&self, skill: SkillDefinition) {
        let name = skill.name.clone();
        self.skills.write().unwrap().insert(name, skill);
    }

    /// Look up a skill by name (case-insensitive).
    pub fn get(&self, name: &str) -> Option<SkillDefinition> {
        let skills = self.skills.read().unwrap();
        skills
            .get(name)
            .or_else(|| {
                let lower = name.to_lowercase();
                skills
                    .iter()
                    .find(|(k, _)| k.to_lowercase() == lower)
                    .map(|(_, v)| v)
            })
            .cloned()
    }

    /// Get all registered skills.
    pub fn all(&self) -> Vec<SkillDefinition> {
        self.skills.read().unwrap().values().cloned().collect()
    }

    /// Get only user-invocable skills.
    pub fn user_invocable(&self) -> Vec<SkillDefinition> {
        self.skills
            .read()
            .unwrap()
            .values()
            .filter(|s| s.user_invocable)
            .cloned()
            .collect()
    }

    /// Number of registered skills.
    pub fn len(&self) -> usize {
        self.skills.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.skills.read().unwrap().is_empty()
    }
}

/// Load skill definitions from a directory.
///
/// Discovers skills from:
/// - `*.md` files directly in the directory (name = filename stem)
/// - `*/skill.md` subdirectories (name = directory name)
pub fn load_skills_dir(dir: &Path, source: SkillSource) -> Vec<SkillDefinition> {
    let mut skills = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return skills,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
            // Direct .md file: commit.md → "commit"
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            if let Some(skill) = load_skill_file(&path, &name, source.clone()) { skills.push(skill) }
        } else if path.is_dir() {
            // Subdirectory: deploy/skill.md → "deploy"
            let skill_file = path.join("skill.md");
            if skill_file.exists() {
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                if let Some(skill) = load_skill_file(&skill_file, &name, source.clone()) { skills.push(skill) }
            }
        }
    }

    skills
}

fn load_skill_file(path: &Path, default_name: &str, source: SkillSource) -> Option<SkillDefinition> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            warn!(path = ?path, error = %e, "failed to read skill file");
            return None;
        }
    };

    match parse_skill_markdown(&content, default_name) {
        Ok(mut skill) => {
            skill.source = source;
            info!(name = %skill.name, path = ?path, "loaded skill");
            Some(skill)
        }
        Err(e) => {
            warn!(path = ?path, error = %e, "failed to parse skill");
            None
        }
    }
}

/// Load and merge skills from project and user directories.
///
/// Project skills take priority over user skills with the same name.
pub fn load_merged_skills(
    project_dir: Option<&Path>,
    user_dir: Option<&Path>,
) -> SkillRegistry {
    let registry = SkillRegistry::new();

    // Load user skills first (lower priority)
    if let Some(dir) = user_dir {
        for skill in load_skills_dir(dir, SkillSource::User) {
            registry.register(skill);
        }
    }

    // Load project skills second (higher priority — overwrites user skills)
    if let Some(dir) = project_dir {
        for skill in load_skills_dir(dir, SkillSource::Project) {
            registry.register(skill);
        }
    }

    registry
}

/// Format skill listing for the system prompt, respecting a character budget.
///
/// Each skill is listed as `- name: description` with descriptions truncated
/// to `max_desc_chars` if needed. Total output is capped at `budget_chars`.
pub fn format_skill_listing(skills: &[SkillDefinition], budget_chars: usize, max_desc_chars: usize) -> String {
    if skills.is_empty() {
        return String::new();
    }

    let mut lines = Vec::new();
    let mut total_chars = 0;
    let header = "The following skills are available for use with the Skill tool:\n";
    total_chars += header.len();

    for skill in skills {
        let desc = if skill.description.len() > max_desc_chars {
            format!("{}...", &skill.description[..max_desc_chars - 3])
        } else {
            skill.description.clone()
        };

        let line = if let Some(ref hint) = skill.argument_hint {
            format!("- {} {}: {}", skill.name, hint, desc)
        } else {
            format!("- {}: {}", skill.name, desc)
        };

        if total_chars + line.len() + 1 > budget_chars {
            break;
        }

        total_chars += line.len() + 1;
        lines.push(line);
    }

    if lines.is_empty() {
        return String::new();
    }

    format!("{}{}", header, lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_skills_from_files() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("commit.md"),
            r#"---
description: "Create a git commit"
---

Review changes and commit.
"#,
        )
        .unwrap();

        std::fs::write(
            dir.path().join("review.md"),
            r#"---
description: "Review code for quality"
---

Review the code.
"#,
        )
        .unwrap();

        let skills = load_skills_dir(dir.path(), SkillSource::Project);
        assert_eq!(skills.len(), 2);

        let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"commit"));
        assert!(names.contains(&"review"));
    }

    #[test]
    fn load_skill_from_subdirectory() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("deploy");
        std::fs::create_dir(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("skill.md"),
            r#"---
description: "Deploy the application"
---

Deploy it.
"#,
        )
        .unwrap();

        let skills = load_skills_dir(dir.path(), SkillSource::Project);
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "deploy");
    }

    #[test]
    fn project_overrides_user() {
        let user_dir = TempDir::new().unwrap();
        std::fs::write(
            user_dir.path().join("commit.md"),
            r#"---
description: "User commit"
---

User version.
"#,
        )
        .unwrap();

        let project_dir = TempDir::new().unwrap();
        std::fs::write(
            project_dir.path().join("commit.md"),
            r#"---
description: "Project commit"
---

Project version.
"#,
        )
        .unwrap();

        let registry = load_merged_skills(Some(project_dir.path()), Some(user_dir.path()));
        let skill = registry.get("commit").unwrap();
        assert_eq!(skill.description, "Project commit");
        assert_eq!(skill.source, SkillSource::Project);
    }

    #[test]
    fn registry_case_insensitive_lookup() {
        let registry = SkillRegistry::new();
        registry.register(SkillDefinition {
            name: "Commit".into(),
            description: "test".into(),
            prompt: "test".into(),
            argument_hint: None,
            allowed_tools: None,
            model: None,
            user_invocable: true,
            source: SkillSource::Project,
        });

        assert!(registry.get("commit").is_some());
        assert!(registry.get("COMMIT").is_some());
        assert!(registry.get("Commit").is_some());
    }

    #[test]
    fn empty_dir_returns_no_skills() {
        let dir = TempDir::new().unwrap();
        let skills = load_skills_dir(dir.path(), SkillSource::Project);
        assert!(skills.is_empty());
    }

    #[test]
    fn nonexistent_dir_returns_no_skills() {
        let skills = load_skills_dir(Path::new("/nonexistent"), SkillSource::Project);
        assert!(skills.is_empty());
    }

    #[test]
    fn format_listing_basic() {
        let skills = vec![
            SkillDefinition {
                name: "commit".into(),
                description: "Create a git commit".into(),
                prompt: String::new(),
                argument_hint: Some("[-m msg]".into()),
                allowed_tools: None,
                model: None,
                user_invocable: true,
                source: SkillSource::Project,
            },
            SkillDefinition {
                name: "review".into(),
                description: "Review code quality".into(),
                prompt: String::new(),
                argument_hint: None,
                allowed_tools: None,
                model: None,
                user_invocable: true,
                source: SkillSource::Project,
            },
        ];

        let listing = format_skill_listing(&skills, 10000, 250);
        assert!(listing.contains("- commit [-m msg]: Create a git commit"));
        assert!(listing.contains("- review: Review code quality"));
    }

    #[test]
    fn format_listing_truncates_description() {
        let skills = vec![SkillDefinition {
            name: "verbose".into(),
            description: "A".repeat(300),
            prompt: String::new(),
            argument_hint: None,
            allowed_tools: None,
            model: None,
            user_invocable: true,
            source: SkillSource::Project,
        }];

        let listing = format_skill_listing(&skills, 10000, 50);
        assert!(listing.contains("..."));
        // Description should be truncated to ~50 chars
        assert!(listing.len() < 200);
    }

    #[test]
    fn format_listing_respects_budget() {
        let skills: Vec<SkillDefinition> = (0..100)
            .map(|i| SkillDefinition {
                name: format!("skill-{i}"),
                description: format!("Description for skill {i}"),
                prompt: String::new(),
                argument_hint: None,
                allowed_tools: None,
                model: None,
                user_invocable: true,
                source: SkillSource::Project,
            })
            .collect();

        let listing = format_skill_listing(&skills, 200, 250);
        assert!(listing.len() <= 200);
    }

    #[test]
    fn format_listing_empty() {
        let listing = format_skill_listing(&[], 10000, 250);
        assert!(listing.is_empty());
    }

    #[test]
    fn user_invocable_filter() {
        let registry = SkillRegistry::new();
        registry.register(SkillDefinition {
            name: "visible".into(),
            description: "test".into(),
            prompt: "test".into(),
            argument_hint: None,
            allowed_tools: None,
            model: None,
            user_invocable: true,
            source: SkillSource::Project,
        });
        registry.register(SkillDefinition {
            name: "hidden".into(),
            description: "test".into(),
            prompt: "test".into(),
            argument_hint: None,
            allowed_tools: None,
            model: None,
            user_invocable: false,
            source: SkillSource::Project,
        });

        let num_builtins = registry.all().iter().filter(|s| s.source == SkillSource::BuiltIn).count();
        assert_eq!(registry.all().len(), 2 + num_builtins);
        let user_invocable = registry.user_invocable();
        assert!(user_invocable.iter().any(|s| s.name == "visible"));
        assert!(!user_invocable.iter().any(|s| s.name == "hidden"));
    }
}
