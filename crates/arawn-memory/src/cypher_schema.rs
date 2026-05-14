//! Schema-as-Rust-types layer between `arawn-memory`'s closed-enum entity model
//! and graphqlite's schemaless EAV storage.
//!
//! graphqlite stores nodes as labeled blobs with arbitrary properties. This
//! module is the single place where:
//!   - `EntityType` is mapped to a Cypher label (`Fact`, `Decision`, …).
//!   - `RelationType` is mapped to a screaming-snake edge type (`RELATES_TO`, …).
//!   - `Entity` scalars are projected into JSON-typed Cypher parameters.
//!   - A returned graphqlite node `Value` is parsed back into an `Entity`.
//!
//! All schema enforcement (valid types, required fields, tag serialization)
//! happens here. Cypher queries against the underlying graph never see raw
//! user input.

use chrono::{DateTime, Utc};
use graphqlite::Value;
use serde_json::{json, Value as JsonValue};
use uuid::Uuid;

use crate::error::MemoryError;
use crate::types::{ConfidenceSource, Entity, EntityType, RelationType};

/// Cypher node label for an `EntityType`.
pub fn entity_label(t: EntityType) -> &'static str {
    match t {
        EntityType::Fact => "Fact",
        EntityType::Decision => "Decision",
        EntityType::Convention => "Convention",
        EntityType::Preference => "Preference",
        EntityType::Person => "Person",
        EntityType::Note => "Note",
    }
}

/// Inverse of `entity_label`.
pub fn entity_type_from_label(s: &str) -> Option<EntityType> {
    match s {
        "Fact" => Some(EntityType::Fact),
        "Decision" => Some(EntityType::Decision),
        "Convention" => Some(EntityType::Convention),
        "Preference" => Some(EntityType::Preference),
        "Person" => Some(EntityType::Person),
        "Note" => Some(EntityType::Note),
        _ => None,
    }
}

/// Cypher relationship type for a `RelationType`.
pub fn relation_type_str(t: RelationType) -> &'static str {
    match t {
        RelationType::RelatesTo => "RELATES_TO",
        RelationType::Contradicts => "CONTRADICTS",
        RelationType::Supports => "SUPPORTS",
        RelationType::Supersedes => "SUPERSEDES",
        RelationType::ExtractedFrom => "EXTRACTED_FROM",
        RelationType::Mentions => "MENTIONS",
        RelationType::BelongsTo => "BELONGS_TO",
        RelationType::Summarizes => "SUMMARIZES",
    }
}

/// Inverse of `relation_type_str`.
pub fn relation_type_from_str(s: &str) -> Option<RelationType> {
    match s {
        "RELATES_TO" => Some(RelationType::RelatesTo),
        "CONTRADICTS" => Some(RelationType::Contradicts),
        "SUPPORTS" => Some(RelationType::Supports),
        "SUPERSEDES" => Some(RelationType::Supersedes),
        "EXTRACTED_FROM" => Some(RelationType::ExtractedFrom),
        "MENTIONS" => Some(RelationType::Mentions),
        "BELONGS_TO" => Some(RelationType::BelongsTo),
        "SUMMARIZES" => Some(RelationType::Summarizes),
        _ => None,
    }
}

/// Project an `Entity` into a Cypher parameter map (`$props`).
///
/// Tags are serialized as a JSON-string property (`tags`). Multi-label was
/// considered and rejected — see ADR-A-0002.
pub fn entity_to_props(e: &Entity) -> JsonValue {
    let tags_json = serde_json::to_string(&e.tags).unwrap_or_else(|_| "[]".into());
    let tags_ontology_json =
        serde_json::to_string(&e.tags_ontology).unwrap_or_else(|_| "[]".into());
    json!({
        "id": e.id.to_string(),
        "title": e.title,
        "content": e.content,
        "confidence_source": e.confidence_source.as_str(),
        "reinforcement_count": e.reinforcement_count as i64,
        "superseded": e.superseded,
        "tags": tags_json,
        "tags_ontology": tags_ontology_json,
        "source_session": e.source_session.map(|u| u.to_string()),
        "created_at": e.created_at.to_rfc3339(),
        "updated_at": e.updated_at.to_rfc3339(),
        "accessed_at": e.accessed_at.to_rfc3339(),
    })
}

/// Parse a node `Value` (as returned by `MATCH (n) RETURN n`) into an `Entity`.
///
/// The node JSON shape is `{id, labels: [...], properties: {...}}`. The first
/// recognized label maps to `EntityType`; properties supply the scalar fields.
pub fn node_to_entity(node: &Value) -> Result<Entity, MemoryError> {
    let labels = node
        .get("labels")
        .and_then(|v| v.as_array())
        .ok_or_else(|| MemoryError::Storage("node missing labels array".into()))?;

    let entity_type = labels
        .iter()
        .filter_map(|v| v.as_str())
        .find_map(entity_type_from_label)
        .ok_or_else(|| MemoryError::Storage("no recognized entity label on node".into()))?;

    let props = node
        .get("properties")
        .and_then(|v| v.as_object())
        .ok_or_else(|| MemoryError::Storage("node missing properties".into()))?;

    let get_str = |k: &str| -> Option<&str> { props.get(k).and_then(|v| v.as_str()) };
    let get_i64 = |k: &str| -> Option<i64> { props.get(k).and_then(|v| v.as_i64()) };
    let get_bool = |k: &str| -> Option<bool> {
        props.get(k).and_then(|v| match v {
            Value::Bool(b) => Some(*b),
            Value::Integer(0) => Some(false),
            Value::Integer(_) => Some(true),
            _ => None,
        })
    };

    let id_str = get_str("id")
        .ok_or_else(|| MemoryError::Storage("node missing 'id' property".into()))?;
    let id = Uuid::parse_str(id_str)
        .map_err(|e| MemoryError::Storage(format!("parse id: {e}")))?;

    let title = get_str("title")
        .ok_or_else(|| MemoryError::Storage("node missing 'title' property".into()))?
        .to_string();

    let content = get_str("content").map(|s| s.to_string());

    let confidence_source = get_str("confidence_source")
        .and_then(ConfidenceSource::from_str)
        .unwrap_or(ConfidenceSource::Inferred);

    let reinforcement_count = get_i64("reinforcement_count").unwrap_or(0).max(0) as u32;
    let superseded = get_bool("superseded").unwrap_or(false);

    let parse_tags = |k: &str| -> Vec<String> {
        match props.get(k) {
            Some(Value::String(s)) => serde_json::from_str(s).unwrap_or_default(),
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
            _ => Vec::new(),
        }
    };
    let tags = parse_tags("tags");
    let tags_ontology = parse_tags("tags_ontology");

    let source_session = get_str("source_session").and_then(|s| Uuid::parse_str(s).ok());

    let parse_dt = |k: &str| -> DateTime<Utc> {
        get_str(k)
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now)
    };

    Ok(Entity {
        id,
        entity_type,
        title,
        content,
        confidence_source,
        reinforcement_count,
        superseded,
        tags,
        tags_ontology,
        source_session,
        created_at: parse_dt("created_at"),
        updated_at: parse_dt("updated_at"),
        accessed_at: parse_dt("accessed_at"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_roundtrip() {
        for et in [
            EntityType::Fact,
            EntityType::Decision,
            EntityType::Convention,
            EntityType::Preference,
            EntityType::Person,
            EntityType::Note,
        ] {
            assert_eq!(entity_type_from_label(entity_label(et)), Some(et));
        }
    }

    #[test]
    fn relation_roundtrip() {
        for rt in [
            RelationType::RelatesTo,
            RelationType::Contradicts,
            RelationType::Supports,
            RelationType::Supersedes,
            RelationType::ExtractedFrom,
            RelationType::Mentions,
            RelationType::BelongsTo,
        ] {
            assert_eq!(relation_type_from_str(relation_type_str(rt)), Some(rt));
        }
    }

    #[test]
    fn entity_to_props_serializes_tags_as_json_string() {
        let e = Entity::new(EntityType::Fact, "Rust is fast")
            .with_tags(vec!["rust".into(), "perf".into()]);
        let props = entity_to_props(&e);
        let tags = props.get("tags").and_then(|v| v.as_str()).unwrap();
        let parsed: Vec<String> = serde_json::from_str(tags).unwrap();
        assert_eq!(parsed, vec!["rust", "perf"]);
    }
}
