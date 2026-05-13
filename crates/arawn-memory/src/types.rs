//! Core types for the knowledge base memory system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type of entity stored in the knowledge base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityType {
    Fact,
    Decision,
    Convention,
    Preference,
    Person,
    Note,
}

impl EntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fact => "fact",
            Self::Decision => "decision",
            Self::Convention => "convention",
            Self::Preference => "preference",
            Self::Person => "person",
            Self::Note => "note",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "fact" => Some(Self::Fact),
            "decision" => Some(Self::Decision),
            "convention" => Some(Self::Convention),
            "preference" => Some(Self::Preference),
            "person" => Some(Self::Person),
            "note" => Some(Self::Note),
            _ => None,
        }
    }

    /// Default scope for this entity type.
    pub fn default_scope(&self) -> Scope {
        match self {
            Self::Preference | Self::Person => Scope::Global,
            Self::Decision | Self::Convention | Self::Note | Self::Fact => Scope::Workstream,
        }
    }
}

/// Which KB tier an entity belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Global,
    Workstream,
}

/// Type of relationship between entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    RelatesTo,
    Contradicts,
    Supports,
    Supersedes,
    ExtractedFrom,
    Mentions,
    BelongsTo,
    /// Edge from a steward-written summary entity to one of the source
    /// entities it summarizes. Allowed by the dust subroutine per
    /// ARAWN-A-0003.
    Summarizes,
}

impl RelationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RelatesTo => "relates_to",
            Self::Contradicts => "contradicts",
            Self::Supports => "supports",
            Self::Supersedes => "supersedes",
            Self::ExtractedFrom => "extracted_from",
            Self::Mentions => "mentions",
            Self::BelongsTo => "belongs_to",
            Self::Summarizes => "summarizes",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "relates_to" => Some(Self::RelatesTo),
            "contradicts" => Some(Self::Contradicts),
            "supports" => Some(Self::Supports),
            "supersedes" => Some(Self::Supersedes),
            "extracted_from" => Some(Self::ExtractedFrom),
            "mentions" => Some(Self::Mentions),
            "belongs_to" => Some(Self::BelongsTo),
            "summarizes" => Some(Self::Summarizes),
            _ => None,
        }
    }
}

/// How confident we are in this entity's accuracy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfidenceSource {
    /// User explicitly stated it (highest confidence).
    Stated,
    /// Observed from behavior/context.
    Observed,
    /// Inferred by extraction pipeline (lowest confidence).
    Inferred,
}

impl ConfidenceSource {
    pub fn base_score(&self) -> f32 {
        match self {
            Self::Stated => 1.0,
            Self::Observed => 0.7,
            Self::Inferred => 0.5,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stated => "stated",
            Self::Observed => "observed",
            Self::Inferred => "inferred",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "stated" => Some(Self::Stated),
            "observed" => Some(Self::Observed),
            "inferred" => Some(Self::Inferred),
            _ => None,
        }
    }
}

/// Compute confidence score with reinforcement and staleness.
pub fn compute_confidence(
    source: ConfidenceSource,
    reinforcement_count: u32,
    days_since_update: f64,
    superseded: bool,
) -> f32 {
    if superseded {
        return 0.0;
    }

    let base = source.base_score();

    // Reinforcement boost: capped at 1.5x
    let reinforcement = (1.0f32 + 0.1 * reinforcement_count as f32).min(1.5);

    // Staleness decay: 1.0 for first 30 days, decays to 0.3 over 365 days
    let staleness = if days_since_update <= 30.0 {
        1.0f32
    } else {
        let decay_days = (days_since_update - 30.0).min(335.0) as f32;
        let decay = decay_days / 335.0;
        1.0 - (decay * 0.7)
    };

    base * reinforcement * staleness
}

/// A knowledge entity stored in the KB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: Uuid,
    pub entity_type: EntityType,
    pub title: String,
    pub content: Option<String>,
    pub confidence_source: ConfidenceSource,
    pub reinforcement_count: u32,
    pub superseded: bool,
    pub tags: Vec<String>,
    pub source_session: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub accessed_at: DateTime<Utc>,
}

impl Entity {
    pub fn new(entity_type: EntityType, title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            entity_type,
            title: title.into(),
            content: None,
            confidence_source: ConfidenceSource::Inferred,
            reinforcement_count: 0,
            superseded: false,
            tags: Vec::new(),
            source_session: None,
            created_at: now,
            updated_at: now,
            accessed_at: now,
        }
    }

    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    pub fn with_confidence(mut self, source: ConfidenceSource) -> Self {
        self.confidence_source = source;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_session(mut self, session_id: Uuid) -> Self {
        self.source_session = Some(session_id);
        self
    }

    /// Compute the current confidence score.
    pub fn confidence_score(&self) -> f32 {
        let days = (Utc::now() - self.updated_at).num_seconds() as f64 / 86400.0;
        compute_confidence(
            self.confidence_source,
            self.reinforcement_count,
            days,
            self.superseded,
        )
    }
}

/// A directed relation between two entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub source_id: Uuid,
    pub relation_type: RelationType,
    pub target_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Result of a store_fact operation (search-before-create).
#[derive(Debug, Clone)]
pub enum StoreFactResult {
    /// New entity created.
    Inserted { entity_id: Uuid },
    /// Existing entity reinforced (same fact seen again).
    Reinforced {
        entity_id: Uuid,
        new_count: u32,
    },
    /// Old entity superseded by new one.
    Superseded {
        old_entity_id: Uuid,
        new_entity_id: Uuid,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_type_roundtrip() {
        for et in [
            EntityType::Fact,
            EntityType::Decision,
            EntityType::Convention,
            EntityType::Preference,
            EntityType::Person,
            EntityType::Note,
        ] {
            assert_eq!(EntityType::from_str(et.as_str()), Some(et));
        }
    }

    #[test]
    fn relation_type_roundtrip() {
        for rt in [
            RelationType::RelatesTo,
            RelationType::Contradicts,
            RelationType::Supports,
            RelationType::Supersedes,
            RelationType::ExtractedFrom,
            RelationType::Mentions,
            RelationType::BelongsTo,
        ] {
            assert_eq!(RelationType::from_str(rt.as_str()), Some(rt));
        }
    }

    #[test]
    fn confidence_stated_fresh() {
        let score = compute_confidence(ConfidenceSource::Stated, 0, 0.0, false);
        assert!((score - 1.0).abs() < 0.01);
    }

    #[test]
    fn confidence_reinforced() {
        let score = compute_confidence(ConfidenceSource::Inferred, 5, 0.0, false);
        // base 0.5 * min(1.5, 1.0 + 0.5) = 0.5 * 1.5 = 0.75
        assert!((score - 0.75).abs() < 0.01);
    }

    #[test]
    fn confidence_stale() {
        let score = compute_confidence(ConfidenceSource::Stated, 0, 365.0, false);
        // base 1.0 * 1.0 * 0.3 = 0.3
        assert!((score - 0.3).abs() < 0.01);
    }

    #[test]
    fn confidence_superseded_is_zero() {
        let score = compute_confidence(ConfidenceSource::Stated, 10, 0.0, true);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn entity_builder() {
        let e = Entity::new(EntityType::Fact, "Rust is fast")
            .with_content("Rust compiles to native code")
            .with_confidence(ConfidenceSource::Stated)
            .with_tags(vec!["rust".into(), "performance".into()]);

        assert_eq!(e.entity_type, EntityType::Fact);
        assert_eq!(e.title, "Rust is fast");
        assert_eq!(e.tags.len(), 2);
        assert!(e.confidence_score() > 0.9);
    }

    #[test]
    fn default_scopes() {
        assert_eq!(EntityType::Preference.default_scope(), Scope::Global);
        assert_eq!(EntityType::Person.default_scope(), Scope::Global);
        assert_eq!(EntityType::Decision.default_scope(), Scope::Workstream);
        assert_eq!(EntityType::Convention.default_scope(), Scope::Workstream);
        assert_eq!(EntityType::Fact.default_scope(), Scope::Workstream);
        assert_eq!(EntityType::Note.default_scope(), Scope::Workstream);
    }
}
