//! Session injection — format KB entities for system prompt context.

use crate::manager::MemoryManager;
use crate::types::EntityType;

/// Default limits for entities injected per tier.
const DEFAULT_GLOBAL_LIMIT: usize = 20;
const DEFAULT_WORKSTREAM_LIMIT: usize = 30;

/// Load relevant entities from both KB tiers and format as strings
/// suitable for system prompt injection.
///
/// Returns a Vec<String> where each entry is a formatted memory line.
/// Empty if no entities exist.
pub fn load_memories_for_injection(
    memory: &MemoryManager,
    global_limit: Option<usize>,
    workstream_limit: Option<usize>,
) -> Vec<String> {
    let global_limit = global_limit.unwrap_or(DEFAULT_GLOBAL_LIMIT);
    let workstream_limit = workstream_limit.unwrap_or(DEFAULT_WORKSTREAM_LIMIT);

    let mut memories = Vec::new();

    // Global KB: all Preferences, high-confidence Facts and Persons
    let global_sections = [
        ("User Preferences", EntityType::Preference),
        ("Known People", EntityType::Person),
        ("Global Facts", EntityType::Fact),
    ];

    let mut global_entries = Vec::new();
    for (label, et) in &global_sections {
        match memory.global.list_by_type(*et, global_limit) {
            Ok(entities) if !entities.is_empty() => {
                // Filter to high-confidence for Facts
                let filtered: Vec<_> = if *et == EntityType::Fact {
                    entities.into_iter().filter(|e| e.confidence_score() > 0.7).collect()
                } else {
                    entities
                };
                if !filtered.is_empty() {
                    global_entries.push(format!("**{}:**", label));
                    for e in &filtered {
                        let line = format_entity_line(e);
                        global_entries.push(line);
                    }
                }
            }
            _ => {}
        }
    }

    if !global_entries.is_empty() {
        memories.push(global_entries.join("\n"));
    }

    // Workstream KB: all Conventions/Decisions, recent Facts/Notes
    let ws_sections = [
        ("Project Conventions", EntityType::Convention),
        ("Project Decisions", EntityType::Decision),
        ("Project Facts", EntityType::Fact),
        ("Notes", EntityType::Note),
    ];

    let mut ws_entries = Vec::new();
    let mut ws_count = 0;
    for (label, et) in &ws_sections {
        if ws_count >= workstream_limit {
            break;
        }
        let remaining = workstream_limit - ws_count;
        match memory.workstream.list_by_type(*et, remaining) {
            Ok(entities) if !entities.is_empty() => {
                ws_entries.push(format!("**{}:**", label));
                for e in &entities {
                    let line = format_entity_line(e);
                    ws_entries.push(line);
                    ws_count += 1;
                }
            }
            _ => {}
        }
    }

    if !ws_entries.is_empty() {
        memories.push(ws_entries.join("\n"));
    }

    memories
}

fn format_entity_line(entity: &crate::types::Entity) -> String {
    let tags = if entity.tags.is_empty() {
        String::new()
    } else {
        format!(" [{}]", entity.tags.join(", "))
    };
    let reinforced = if entity.reinforcement_count > 0 {
        format!(" ({}x confirmed)", entity.reinforcement_count)
    } else {
        String::new()
    };
    let snippet = entity
        .content
        .as_deref()
        .map(|c| {
            let s: String = c.chars().take(100).collect();
            format!(" — {s}")
        })
        .unwrap_or_default();

    format!("- {}{snippet}{reinforced}{tags}", entity.title)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, MemoryManager) {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("workstreams/test-ws")).unwrap();
        let mgr = MemoryManager::open(tmp.path(), "test-ws", None).unwrap();
        (tmp, mgr)
    }

    #[test]
    fn empty_kb_returns_empty() {
        let (_tmp, mgr) = setup();
        let memories = load_memories_for_injection(&mgr, None, None);
        assert!(memories.is_empty());
    }

    #[test]
    fn injects_global_preferences() {
        let (_tmp, mgr) = setup();
        mgr.global
            .insert_entity(
                &Entity::new(EntityType::Preference, "Terse responses")
                    .with_confidence(ConfidenceSource::Stated),
            )
            .unwrap();

        let memories = load_memories_for_injection(&mgr, None, None);
        assert!(!memories.is_empty());
        let joined = memories.join("\n");
        assert!(joined.contains("Terse responses"));
        assert!(joined.contains("User Preferences"));
    }

    #[test]
    fn injects_workstream_conventions() {
        let (_tmp, mgr) = setup();
        mgr.workstream
            .insert_entity(&Entity::new(EntityType::Convention, "Tests go inline"))
            .unwrap();
        mgr.workstream
            .insert_entity(&Entity::new(EntityType::Decision, "Use microservices"))
            .unwrap();

        let memories = load_memories_for_injection(&mgr, None, None);
        let joined = memories.join("\n");
        assert!(joined.contains("Tests go inline"));
        assert!(joined.contains("Use microservices"));
        assert!(joined.contains("Project Conventions"));
        assert!(joined.contains("Project Decisions"));
    }

    #[test]
    fn both_tiers_injected() {
        let (_tmp, mgr) = setup();
        mgr.global
            .insert_entity(&Entity::new(EntityType::Preference, "Likes Rust"))
            .unwrap();
        mgr.workstream
            .insert_entity(&Entity::new(EntityType::Decision, "Use PostgreSQL"))
            .unwrap();

        let memories = load_memories_for_injection(&mgr, None, None);
        assert_eq!(memories.len(), 2); // one global block, one workstream block
    }

    #[test]
    fn reinforcement_shown() {
        let (_tmp, mgr) = setup();
        let mut entity = Entity::new(EntityType::Fact, "Rust is fast");
        entity.reinforcement_count = 3;
        mgr.workstream.insert_entity(&entity).unwrap();

        let memories = load_memories_for_injection(&mgr, None, None);
        let joined = memories.join("\n");
        assert!(joined.contains("3x confirmed"));
    }
}
