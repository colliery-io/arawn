//! Sidecar profile for Person entities — structured fields layered on top of
//! the graphqlite Entity model.
//!
//! ARAWN-I-0064 T-B: `Entity` itself stays a thin closed-enum record (per the
//! graphqlite EAV model); rich Person-specific fields live in a separate
//! SQLite table keyed by `entity_id`. A Person entity exists with no profile
//! row until the first structured field is written — `get_person_profile`
//! returns `None`, callers fall back to the title/content on the Entity.
//!
//! Architectural note (I-0064 design): the user themselves is *not* modeled
//! as a Person entity. Their relations to other people are captured via the
//! `relation_to_user` column on this profile. Person↔Person relations
//! between two non-self people use the typed `RelationType::{Manages,
//! ReportsTo, PeerOf}` graph edges added in T-A.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// How the user is connected to this Person. Special because the user
/// themselves isn't a Person entity; their org-graph position is encoded
/// on the other people's profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationToUser {
    /// User manages this person (direct report).
    Manages,
    /// This person is the user's manager.
    ReportsToUser,
    /// This person is a peer of the user (same level, not in user's org).
    PeerOfUser,
}

impl RelationToUser {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manages => "manages",
            Self::ReportsToUser => "reports_to_user",
            Self::PeerOfUser => "peer_of_user",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "manages" => Some(Self::Manages),
            "reports_to_user" => Some(Self::ReportsToUser),
            "peer_of_user" => Some(Self::PeerOfUser),
            _ => None,
        }
    }
}

/// Structured profile for a Person entity. All fields beyond `entity_id`
/// are optional — profile rows only exist once at least one structured
/// fact about the person is captured. Read-time callers fall back to the
/// Entity's `title` / `content` when a profile is absent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonProfile {
    /// App-level FK into the graphqlite Entity model. SQLite-level FK
    /// can't reach into graphqlite's EAV; integrity is enforced by
    /// `MemoryStore::delete_entity` cascading into this table.
    pub entity_id: Uuid,
    pub role: Option<String>,
    pub relation_to_user: Option<RelationToUser>,
    /// FK → another Person's entity_id. Used for Manages/ReportsTo
    /// chains between two non-self people; the graph edge in
    /// `RelationType` is the canonical source, this column is a
    /// query-friendly denormalization.
    pub reports_to_person_id: Option<Uuid>,
    pub hire_date: Option<NaiveDate>,
    pub last_1on1: Option<DateTime<Utc>>,
    pub pronouns: Option<String>,
    pub time_zone: Option<String>,
    pub growth_areas: Vec<String>,
    pub current_concerns: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PersonProfile {
    /// Construct a new profile for the given Person entity with all
    /// optional fields empty. Timestamps default to now.
    pub fn new(entity_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            entity_id,
            role: None,
            relation_to_user: None,
            reports_to_person_id: None,
            hire_date: None,
            last_1on1: None,
            pronouns: None,
            time_zone: None,
            growth_areas: Vec::new(),
            current_concerns: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }

    pub fn with_relation_to_user(mut self, rel: RelationToUser) -> Self {
        self.relation_to_user = Some(rel);
        self
    }

    pub fn with_reports_to(mut self, person_id: Uuid) -> Self {
        self.reports_to_person_id = Some(person_id);
        self
    }

    pub fn with_last_1on1(mut self, ts: DateTime<Utc>) -> Self {
        self.last_1on1 = Some(ts);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relation_to_user_roundtrip() {
        for rel in [
            RelationToUser::Manages,
            RelationToUser::ReportsToUser,
            RelationToUser::PeerOfUser,
        ] {
            assert_eq!(RelationToUser::from_str(rel.as_str()), Some(rel));
        }
    }

    #[test]
    fn builder_fills_only_set_fields() {
        let id = Uuid::new_v4();
        let p = PersonProfile::new(id)
            .with_role("Senior EM")
            .with_relation_to_user(RelationToUser::Manages);
        assert_eq!(p.entity_id, id);
        assert_eq!(p.role.as_deref(), Some("Senior EM"));
        assert_eq!(p.relation_to_user, Some(RelationToUser::Manages));
        assert!(p.pronouns.is_none());
        assert!(p.growth_areas.is_empty());
    }
}
