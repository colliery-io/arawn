//! Layered memory stack — generates token-budgeted context from the KB.
//!
//! L0: Identity (~100 tokens) — lens metadata, people, conventions
//! L1: Essential facts (~500-800 tokens) — top-ranked entities grouped by type
//! L2: On-demand — topic-triggered retrieval (separate method)

use crate::manager::MemoryManager;
use crate::person_profile::{PersonProfile, RelationToUser};
use crate::types::{Entity, EntityType};

/// Estimate token count from text length (matches arawn-engine's TokenEstimator).
fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

/// Layered memory stack. Call `wake_up()` per-message to get fresh L0+L1 context.
pub struct MemoryStack<'a> {
    manager: &'a MemoryManager,
    lens_name: String,
}

impl<'a> MemoryStack<'a> {
    pub fn new(manager: &'a MemoryManager, lens_name: &str) -> Self {
        Self {
            manager,
            lens_name: lens_name.to_string(),
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

    /// L0: Identity layer — lens name + structured org slice + conventions.
    ///
    /// ARAWN-I-0064 T-D: Person entities are now rendered grouped by the
    /// user's relation to them (`PersonProfile.relation_to_user`) so the
    /// agent sees real org context instead of a flat name list:
    ///
    /// ```text
    /// you manage: Sarah Lee (Senior EM), Marcus (Staff Eng)
    /// you report to: David Chen (VP)
    /// peers: Anita, Priya
    /// people: Pat Collins
    /// ```
    ///
    /// Persons without a `PersonProfile` row (legacy / unstructured
    /// captures) still appear in the plain `people:` fallback so old data
    /// keeps surfacing. Each bucket is capped at 5 names.
    fn render_l0(&self) -> String {
        let mut out = format!("[L0 — IDENTITY] lens: {}\n", self.lens_name);

        // Pull structured org buckets in priority order: directs first,
        // then managers, then peers. Profile rows already come back
        // sorted by `updated_at DESC` from the store.
        let directs = self
            .manager
            .global
            .list_person_profiles_by_relation_to_user(RelationToUser::Manages)
            .unwrap_or_default();
        let managers = self
            .manager
            .global
            .list_person_profiles_by_relation_to_user(RelationToUser::ReportsToUser)
            .unwrap_or_default();
        let peers = self
            .manager
            .global
            .list_person_profiles_by_relation_to_user(RelationToUser::PeerOfUser)
            .unwrap_or_default();

        // Track ids already shown so the legacy `people:` fallback below
        // doesn't double-print directs/managers/peers.
        let mut accounted: std::collections::HashSet<uuid::Uuid> = std::collections::HashSet::new();
        for p in directs.iter().chain(managers.iter()).chain(peers.iter()) {
            accounted.insert(p.entity_id);
        }

        if let Some(line) = self.render_relation_bucket("you manage", &directs) {
            out.push_str(&line);
        }
        if let Some(line) = self.render_relation_bucket("you report to", &managers) {
            out.push_str(&line);
        }
        if let Some(line) = self.render_relation_bucket("peers", &peers) {
            out.push_str(&line);
        }

        // Legacy/unstructured Persons — those without a PersonProfile row.
        // Pull a larger window than the cap so we can filter accounted
        // ids and still produce up to 5 names.
        if let Ok(people) = self.manager.global.list_by_type(EntityType::Person, 20) {
            let leftover: Vec<&str> = people
                .iter()
                .filter(|e| !accounted.contains(&e.id))
                .map(|e| e.title.as_str())
                .take(5)
                .collect();
            if !leftover.is_empty() {
                out.push_str(&format!("people: {}\n", leftover.join(", ")));
            }
        }

        // Core conventions from lens KB
        if let Ok(conventions) = self.manager.lens.list_by_type(EntityType::Convention, 3) {
            for c in &conventions {
                out.push_str(&format!("convention: {}\n", c.title));
            }
        }

        out
    }

    /// Render one relation bucket — "you manage: Sarah (Senior EM), Marcus".
    /// Caps at 5 entries to keep L0 within budget; skips empty buckets so
    /// the L0 output doesn't carry placeholder lines for unused tiers.
    fn render_relation_bucket(&self, label: &str, profiles: &[PersonProfile]) -> Option<String> {
        if profiles.is_empty() {
            return None;
        }
        let parts: Vec<String> = profiles
            .iter()
            .take(5)
            .map(|p| self.format_person_with_role(p))
            .collect();
        Some(format!("{}: {}\n", label, parts.join(", ")))
    }

    /// "Sarah Lee (Senior EM)" when the role column is set; bare title
    /// otherwise. Falls back to "?" if the Entity row vanished
    /// underneath us (shouldn't happen — cascade-delete in T-B keeps
    /// the sidecar in sync — but defensive against partial-state reads).
    fn format_person_with_role(&self, profile: &PersonProfile) -> String {
        let title = self
            .manager
            .global
            .get_entity(profile.entity_id)
            .ok()
            .flatten()
            .map(|e| e.title)
            .unwrap_or_else(|| "?".to_string());
        match profile.role.as_deref() {
            Some(role) if !role.is_empty() => format!("{title} ({role})"),
            _ => title,
        }
    }

    /// L1: Essential story — top-ranked entities grouped by type, within budget.
    /// Returns (rendered text, entity titles included) for shortcode compression.
    fn render_l1_with_names(&self, budget_tokens: usize) -> (String, Vec<String>) {
        // Gather ranked entities from both tiers
        let global = self.manager.global.list_all_ranked(30).unwrap_or_default();
        let lens = self.manager.lens.list_all_ranked(50).unwrap_or_default();

        // Merge and re-sort by confidence score (descending)
        let mut all: Vec<Entity> = global.into_iter().chain(lens).collect();
        all.sort_by(|a, b| {
            b.confidence_score()
                .partial_cmp(&a.confidence_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Deduplicate against L0 entities (Person/Convention already shown)
        let l0_types = [EntityType::Person, EntityType::Convention];

        // Group by type and render within budget
        let mut sections: std::collections::BTreeMap<&str, Vec<String>> =
            std::collections::BTreeMap::new();
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
        let lens = self.manager.lens.list_all_ranked(50).unwrap_or_default();

        let mut all: Vec<Entity> = global.into_iter().chain(lens).collect();
        all.sort_by(|a, b| {
            b.confidence_score()
                .partial_cmp(&a.confidence_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        all.iter()
            .filter(|e| {
                e.entity_type != EntityType::Person && e.entity_type != EntityType::Convention
            })
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
        let l1_set: std::collections::HashSet<&str> =
            l1_titles.iter().map(|s| s.as_str()).collect();
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
        std::fs::create_dir_all(tmp.path().join("lenses/test-ws")).unwrap();
        let mgr = MemoryManager::open(tmp.path(), "test-ws", None).unwrap();
        (tmp, mgr)
    }

    #[test]
    fn wake_up_respects_budget() {
        let (_tmp, mgr) = setup();
        // Add many entities
        for i in 0..50 {
            let mut e = Entity::new(
                EntityType::Fact,
                &format!("Fact number {i} with some extra text to fill tokens"),
            );
            e.content = Some(format!(
                "Content for fact {i} that adds more tokens to the output"
            ));
            mgr.lens.insert_entity(&e).unwrap();
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
        mgr.lens.insert_entity(&inferred).unwrap();

        let mut stated = Entity::new(EntityType::Fact, "Stated fact");
        stated.confidence_source = ConfidenceSource::Stated;
        mgr.lens.insert_entity(&stated).unwrap();

        let stack = MemoryStack::new(&mgr, "test-ws");
        let output = stack.wake_up(900);

        // Stated should appear before inferred in the output
        let stated_pos = output.find("Stated fact").unwrap_or(usize::MAX);
        let inferred_pos = output.find("Inferred fact").unwrap_or(usize::MAX);
        assert!(
            stated_pos < inferred_pos,
            "stated should come before inferred"
        );
    }

    #[test]
    fn tiny_budget_does_not_panic() {
        let (_tmp, mgr) = setup();
        mgr.lens
            .insert_entity(&Entity::new(EntityType::Fact, "Some fact"))
            .unwrap();

        let stack = MemoryStack::new(&mgr, "test-ws");
        let output = stack.wake_up(10); // absurdly small
        assert!(!output.is_empty());
    }

    // === ARAWN-I-0064 T-D: structured L0 org slice ===

    fn make_person(mgr: &MemoryManager, name: &str) -> uuid::Uuid {
        let e = Entity::new(EntityType::Person, name);
        let id = e.id;
        mgr.global.insert_entity(&e).unwrap();
        id
    }

    fn upsert_profile(
        mgr: &MemoryManager,
        entity_id: uuid::Uuid,
        rel: RelationToUser,
        role: Option<&str>,
    ) {
        let mut p = PersonProfile::new(entity_id).with_relation_to_user(rel);
        if let Some(r) = role {
            p = p.with_role(r);
        }
        mgr.global.upsert_person_profile(&p).unwrap();
    }

    #[test]
    fn l0_groups_persons_by_relation_to_user_with_roles() {
        let (_tmp, mgr) = setup();
        let sarah = make_person(&mgr, "Sarah Lee");
        let marcus = make_person(&mgr, "Marcus");
        let david = make_person(&mgr, "David Chen");
        let anita = make_person(&mgr, "Anita");
        upsert_profile(&mgr, sarah, RelationToUser::Manages, Some("Senior EM"));
        upsert_profile(&mgr, marcus, RelationToUser::Manages, Some("Staff Eng"));
        upsert_profile(&mgr, david, RelationToUser::ReportsToUser, Some("VP"));
        upsert_profile(&mgr, anita, RelationToUser::PeerOfUser, None);

        let stack = MemoryStack::new(&mgr, "test-ws");
        let out = stack.wake_up(900);

        // Profiles come back ordered by updated_at DESC (most recent first),
        // so the assertion is order-agnostic — what matters is that both
        // directs land in the "you manage" bucket with their roles.
        let manage_line = out
            .lines()
            .find(|l| l.starts_with("you manage: "))
            .unwrap_or_else(|| panic!("no `you manage` line in:\n{out}"));
        assert!(
            manage_line.contains("Sarah Lee (Senior EM)"),
            "got: {manage_line}"
        );
        assert!(
            manage_line.contains("Marcus (Staff Eng)"),
            "got: {manage_line}"
        );
        assert!(
            out.contains("you report to: David Chen (VP)"),
            "got:\n{out}"
        );
        assert!(out.contains("peers: Anita"), "got:\n{out}");
        // No leftover plain `people:` line — every Person is accounted for.
        assert!(!out.contains("\npeople:"), "got:\n{out}");
    }

    #[test]
    fn l0_falls_back_to_plain_people_line_for_unstructured_persons() {
        let (_tmp, mgr) = setup();
        // Two persons with no PersonProfile — pure legacy capture.
        make_person(&mgr, "Pat Collins");
        make_person(&mgr, "Quinn");

        let stack = MemoryStack::new(&mgr, "test-ws");
        let out = stack.wake_up(900);

        assert!(out.contains("people: "), "got:\n{out}");
        assert!(out.contains("Pat Collins"), "got:\n{out}");
        assert!(out.contains("Quinn"), "got:\n{out}");
        // None of the structured headers should appear.
        assert!(!out.contains("you manage"), "got:\n{out}");
        assert!(!out.contains("you report to"), "got:\n{out}");
        assert!(!out.contains("peers:"), "got:\n{out}");
    }

    #[test]
    fn l0_mixes_structured_and_unstructured_persons() {
        let (_tmp, mgr) = setup();
        let sarah = make_person(&mgr, "Sarah");
        upsert_profile(&mgr, sarah, RelationToUser::Manages, None);
        make_person(&mgr, "Pat Collins"); // unstructured

        let stack = MemoryStack::new(&mgr, "test-ws");
        let out = stack.wake_up(900);

        assert!(out.contains("you manage: Sarah"), "got:\n{out}");
        assert!(out.contains("people: Pat Collins"), "got:\n{out}");
        // Sarah should NOT appear in the legacy `people:` line.
        let people_line = out
            .lines()
            .find(|l| l.starts_with("people: "))
            .expect("people: line present");
        assert!(!people_line.contains("Sarah"), "people line leaked Sarah");
    }

    #[test]
    fn l0_caps_each_bucket_at_five_entries() {
        let (_tmp, mgr) = setup();
        for i in 0..8 {
            let p = make_person(&mgr, &format!("Direct{i}"));
            upsert_profile(&mgr, p, RelationToUser::Manages, None);
        }
        let stack = MemoryStack::new(&mgr, "test-ws");
        let out = stack.wake_up(900);

        let manage_line = out
            .lines()
            .find(|l| l.starts_with("you manage: "))
            .expect("you manage: line present");
        // Comma-separated count of names: 5 names = 4 commas.
        let commas = manage_line.matches(", ").count();
        assert_eq!(commas, 4, "got line: {manage_line}");
    }

    #[test]
    fn l0_person_without_role_renders_bare_name() {
        let (_tmp, mgr) = setup();
        let id = make_person(&mgr, "Anita");
        upsert_profile(&mgr, id, RelationToUser::PeerOfUser, None);

        let stack = MemoryStack::new(&mgr, "test-ws");
        let out = stack.wake_up(900);

        // No parentheses around the name when no role is set.
        assert!(out.contains("peers: Anita\n"), "got:\n{out}");
        assert!(!out.contains("Anita ("), "got:\n{out}");
    }
}
