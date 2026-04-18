//! Layered memory stack — generates token-budgeted context from the KB.
//!
//! L0: Identity (~100 tokens) — workstream metadata, people, conventions
//! L1: Essential facts (~500-800 tokens) — top-ranked entities grouped by type
//! L2: On-demand — topic-triggered retrieval (separate method)

use crate::manager::MemoryManager;
use crate::types::{Entity, EntityType};

/// Estimate token count from text length (matches arawn-engine's TokenEstimator).
fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

/// Layered memory stack. Call `wake_up()` per-message to get fresh L0+L1 context.
pub struct MemoryStack<'a> {
    manager: &'a MemoryManager,
    workstream_name: String,
}

impl<'a> MemoryStack<'a> {
    pub fn new(manager: &'a MemoryManager, workstream_name: &str) -> Self {
        Self {
            manager,
            workstream_name: workstream_name.to_string(),
        }
    }

    /// Generate L0 + L1 memory context within the given token budget.
    /// Returns formatted text ready for system prompt injection.
    pub fn wake_up(&self, budget_tokens: usize) -> String {
        let l0 = self.render_l0();
        let l0_tokens = estimate_tokens(&l0);

        if l0_tokens >= budget_tokens {
            // Budget too small for even L0 — truncate
            let char_budget = budget_tokens * 4;
            return l0.chars().take(char_budget).collect();
        }

        // Reserve 5% for shortcode legend overhead
        let remaining = (budget_tokens - l0_tokens).saturating_sub(budget_tokens / 20);
        let (l1, l1_entity_names) = self.render_l1_with_names(remaining);

        if l1.is_empty() {
            l0
        } else {
            // Apply shortcode compression to L1
            let compressed = crate::shortcodes::apply_shortcodes(&l1, &l1_entity_names, 2);
            format!("{l0}\n{compressed}")
        }
    }

    /// L0: Identity layer — workstream name + Person/Convention entities.
    fn render_l0(&self) -> String {
        let mut out = format!("[L0 — IDENTITY] workstream: {}\n", self.workstream_name);

        // People from global KB
        if let Ok(people) = self.manager.global.list_by_type(EntityType::Person, 5)
            && !people.is_empty() {
                let names: Vec<&str> = people.iter().map(|e| e.title.as_str()).collect();
                out.push_str(&format!("people: {}\n", names.join(", ")));
            }

        // Core conventions from workstream KB
        if let Ok(conventions) = self.manager.workstream.list_by_type(EntityType::Convention, 3) {
            for c in &conventions {
                out.push_str(&format!("convention: {}\n", c.title));
            }
        }

        out
    }

    /// L1: Essential story — top-ranked entities grouped by type, within budget.
    /// Returns (rendered text, entity titles included) for shortcode compression.
    fn render_l1_with_names(&self, budget_tokens: usize) -> (String, Vec<String>) {
        // Gather ranked entities from both tiers
        let global = self.manager.global.list_all_ranked(30).unwrap_or_default();
        let workstream = self.manager.workstream.list_all_ranked(50).unwrap_or_default();

        // Merge and re-sort by confidence score (descending)
        let mut all: Vec<Entity> = global.into_iter().chain(workstream).collect();
        all.sort_by(|a, b| b.confidence_score().partial_cmp(&a.confidence_score()).unwrap_or(std::cmp::Ordering::Equal));

        // Deduplicate against L0 entities (Person/Convention already shown)
        let l0_types = [EntityType::Person, EntityType::Convention];

        // Group by type and render within budget
        let mut sections: std::collections::BTreeMap<&str, Vec<String>> = std::collections::BTreeMap::new();
        let mut entity_names: Vec<String> = Vec::new();
        let mut total_tokens = 20; // header overhead

        for entity in &all {
            // Skip entities already in L0
            if l0_types.contains(&entity.entity_type) {
                continue;
            }

            let line = format_entity_brief(entity);
            let line_tokens = estimate_tokens(&line);

            if total_tokens + line_tokens > budget_tokens {
                break;
            }

            let type_label = entity.entity_type.as_str();
            sections.entry(type_label).or_default().push(line);
            entity_names.push(entity.title.clone());
            total_tokens += line_tokens;
        }

        if sections.is_empty() {
            return (String::new(), vec![]);
        }

        let mut out = String::from("[L1 — KEY FACTS]\n");
        for (label, lines) in &sections {
            out.push_str(&format!("[{label}] "));
            out.push_str(&lines.join(" | "));
            out.push('\n');
        }

        (out, entity_names)
    }

    /// Get the entity titles included in L1 (for L2 deduplication).
    pub fn l1_entity_titles(&self) -> Vec<String> {
        let global = self.manager.global.list_all_ranked(30).unwrap_or_default();
        let workstream = self.manager.workstream.list_all_ranked(50).unwrap_or_default();

        let mut all: Vec<Entity> = global.into_iter().chain(workstream).collect();
        all.sort_by(|a, b| b.confidence_score().partial_cmp(&a.confidence_score()).unwrap_or(std::cmp::Ordering::Equal));

        all.iter()
            .filter(|e| e.entity_type != EntityType::Person && e.entity_type != EntityType::Convention)
            .take(50)
            .map(|e| e.title.clone())
            .collect()
    }

    /// L2: Topic-triggered context. Searches KB for entities matching keywords,
    /// deduplicates against L1, returns formatted section within budget.
    pub fn topical_context(
        &self,
        keywords: &[String],
        l1_titles: &[String],
        budget_tokens: usize,
    ) -> Option<String> {
        let entities = self.manager.retrieve_topical(keywords, budget_tokens);

        // Deduplicate against L1
        let l1_set: std::collections::HashSet<&str> = l1_titles.iter().map(|s| s.as_str()).collect();
        let unique: Vec<&Entity> = entities
            .iter()
            .filter(|e| !l1_set.contains(e.title.as_str()))
            .collect();

        if unique.is_empty() {
            return None;
        }

        let mut out = String::from("[L2 — CONTEXT]\n");
        for entity in &unique {
            out.push_str(&format!("- {}", format_entity_brief(entity)));
            out.push('\n');
        }

        Some(out)
    }
}

fn format_entity_brief(entity: &Entity) -> String {
    let snippet = entity
        .content
        .as_deref()
        .map(|c| {
            let s: String = c.chars().take(80).collect();
            format!(" — {s}")
        })
        .unwrap_or_default();
    format!("{}{snippet}", entity.title)
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
    fn wake_up_respects_budget() {
        let (_tmp, mgr) = setup();
        // Add many entities
        for i in 0..50 {
            let mut e = Entity::new(EntityType::Fact, &format!("Fact number {i} with some extra text to fill tokens"));
            e.content = Some(format!("Content for fact {i} that adds more tokens to the output"));
            mgr.workstream.insert_entity(&e).unwrap();
        }

        let stack = MemoryStack::new(&mgr, "test-ws");
        let output = stack.wake_up(200); // small budget
        let tokens = estimate_tokens(&output);
        assert!(tokens <= 200, "output {tokens} tokens exceeds budget 200");
    }

    #[test]
    fn wake_up_empty_kb() {
        let (_tmp, mgr) = setup();
        let stack = MemoryStack::new(&mgr, "test-ws");
        let output = stack.wake_up(900);
        assert!(output.contains("[L0"));
        assert!(output.contains("test-ws"));
        assert!(!output.contains("[L1")); // no entities = no L1
    }

    #[test]
    fn l1_ranks_stated_before_inferred() {
        let (_tmp, mgr) = setup();

        let mut inferred = Entity::new(EntityType::Fact, "Inferred fact");
        inferred.confidence_source = ConfidenceSource::Inferred;
        mgr.workstream.insert_entity(&inferred).unwrap();

        let mut stated = Entity::new(EntityType::Fact, "Stated fact");
        stated.confidence_source = ConfidenceSource::Stated;
        mgr.workstream.insert_entity(&stated).unwrap();

        let stack = MemoryStack::new(&mgr, "test-ws");
        let output = stack.wake_up(900);

        // Stated should appear before inferred in the output
        let stated_pos = output.find("Stated fact").unwrap_or(usize::MAX);
        let inferred_pos = output.find("Inferred fact").unwrap_or(usize::MAX);
        assert!(stated_pos < inferred_pos, "stated should come before inferred");
    }

    #[test]
    fn tiny_budget_does_not_panic() {
        let (_tmp, mgr) = setup();
        mgr.workstream
            .insert_entity(&Entity::new(EntityType::Fact, "Some fact"))
            .unwrap();

        let stack = MemoryStack::new(&mgr, "test-ws");
        let output = stack.wake_up(10); // absurdly small
        assert!(!output.is_empty());
    }
}
