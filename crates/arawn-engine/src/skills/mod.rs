//! Skill framework — reusable prompt-based workflows.
//!
//! Skills are markdown files with YAML frontmatter that define prompt templates
//! invoked by name (like slash commands). The model calls the SkillTool to
//! execute a skill, which injects the skill's prompt into the conversation.

mod definition;
mod loader;

pub use definition::{SkillDefinition, SkillSource, parse_skill_markdown};
pub use loader::{
    SkillRegistry, format_skill_listing, load_merged_skills, load_skills_dir,
};
