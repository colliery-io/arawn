//! SQLite-backed knowledge base store with FTS5 search and relations.

use std::path::Path;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use tracing::{debug, info};
use uuid::Uuid;

use crate::error::MemoryError;
use crate::types::*;
use crate::vector;

/// Knowledge base store backed by SQLite with FTS5 and relations.
pub struct MemoryStore {
    conn: Mutex<Connection>,
}

impl MemoryStore {
    /// Open or create a memory database at the given path.
    pub fn open(path: &Path) -> Result<Self, MemoryError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| MemoryError::Storage(format!("create dir: {e}")))?;
        }

        let conn = Connection::open(path)
            .map_err(|e| MemoryError::Storage(format!("open db: {e}")))?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| MemoryError::Storage(format!("pragmas: {e}")))?;

        let store = Self {
            conn: Mutex::new(conn),
        };
        store.migrate()?;
        info!(path = ?path, "memory store opened");
        Ok(store)
    }

    /// Create an in-memory store (for testing).
    pub fn in_memory() -> Result<Self, MemoryError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| MemoryError::Storage(format!("open in-memory: {e}")))?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<(), MemoryError> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS entities (
                id TEXT PRIMARY KEY,
                entity_type TEXT NOT NULL,
                title TEXT NOT NULL,
                content TEXT,
                confidence_source TEXT NOT NULL DEFAULT 'inferred',
                reinforcement_count INTEGER NOT NULL DEFAULT 0,
                superseded INTEGER NOT NULL DEFAULT 0,
                tags TEXT NOT NULL DEFAULT '[]',
                source_session TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                accessed_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS relations (
                source_id TEXT NOT NULL,
                relation_type TEXT NOT NULL,
                target_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (source_id, relation_type, target_id)
            );

            CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);
            CREATE INDEX IF NOT EXISTS idx_entities_superseded ON entities(superseded);
            CREATE INDEX IF NOT EXISTS idx_relations_source ON relations(source_id);
            CREATE INDEX IF NOT EXISTS idx_relations_target ON relations(target_id);",
        )
        .map_err(|e| MemoryError::Storage(format!("migrate: {e}")))?;

        // FTS5 virtual table (fails silently if already exists)
        let _ = conn.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS entities_fts USING fts5(
                title, content, content=entities, content_rowid=rowid
            );

            CREATE TRIGGER IF NOT EXISTS entities_ai AFTER INSERT ON entities BEGIN
                INSERT INTO entities_fts(rowid, title, content)
                VALUES (new.rowid, new.title, new.content);
            END;

            CREATE TRIGGER IF NOT EXISTS entities_ad AFTER DELETE ON entities BEGIN
                INSERT INTO entities_fts(entities_fts, rowid, title, content)
                VALUES ('delete', old.rowid, old.title, old.content);
            END;

            CREATE TRIGGER IF NOT EXISTS entities_au AFTER UPDATE ON entities BEGIN
                INSERT INTO entities_fts(entities_fts, rowid, title, content)
                VALUES ('delete', old.rowid, old.title, old.content);
                INSERT INTO entities_fts(rowid, title, content)
                VALUES (new.rowid, new.title, new.content);
            END;",
        );

        Ok(())
    }

    // === Entity CRUD ===

    pub fn insert_entity(&self, entity: &Entity) -> Result<(), MemoryError> {
        let conn = self.conn.lock().unwrap();
        let tags_json = serde_json::to_string(&entity.tags).unwrap_or_else(|_| "[]".into());

        conn.execute(
            "INSERT INTO entities (id, entity_type, title, content, confidence_source,
             reinforcement_count, superseded, tags, source_session, created_at, updated_at, accessed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                entity.id.to_string(),
                entity.entity_type.as_str(),
                entity.title,
                entity.content,
                entity.confidence_source.as_str(),
                entity.reinforcement_count,
                entity.superseded as i32,
                tags_json,
                entity.source_session.map(|u| u.to_string()),
                entity.created_at.to_rfc3339(),
                entity.updated_at.to_rfc3339(),
                entity.accessed_at.to_rfc3339(),
            ],
        )
        .map_err(|e| MemoryError::Storage(format!("insert entity: {e}")))?;

        debug!(id = %entity.id, title = %entity.title, "entity inserted");
        Ok(())
    }

    pub fn get_entity(&self, id: Uuid) -> Result<Option<Entity>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT * FROM entities WHERE id = ?1")
            .map_err(|e| MemoryError::Storage(format!("prepare: {e}")))?;

        let result = stmt
            .query_row(params![id.to_string()], |row| Ok(row_to_entity(row)))
            .optional()
            .map_err(|e| MemoryError::Storage(format!("get entity: {e}")))?;

        match result {
            Some(Ok(entity)) => Ok(Some(entity)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    pub fn update_entity(&self, entity: &Entity) -> Result<(), MemoryError> {
        let conn = self.conn.lock().unwrap();
        let tags_json = serde_json::to_string(&entity.tags).unwrap_or_else(|_| "[]".into());

        conn.execute(
            "UPDATE entities SET title=?2, content=?3, confidence_source=?4,
             reinforcement_count=?5, superseded=?6, tags=?7, updated_at=?8, accessed_at=?9
             WHERE id=?1",
            params![
                entity.id.to_string(),
                entity.title,
                entity.content,
                entity.confidence_source.as_str(),
                entity.reinforcement_count,
                entity.superseded as i32,
                tags_json,
                entity.updated_at.to_rfc3339(),
                entity.accessed_at.to_rfc3339(),
            ],
        )
        .map_err(|e| MemoryError::Storage(format!("update entity: {e}")))?;
        Ok(())
    }

    pub fn delete_entity(&self, id: Uuid) -> Result<bool, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let deleted = conn
            .execute("DELETE FROM entities WHERE id = ?1", params![id.to_string()])
            .map_err(|e| MemoryError::Storage(format!("delete entity: {e}")))?;

        // Clean up relations
        let _ = conn.execute(
            "DELETE FROM relations WHERE source_id = ?1 OR target_id = ?1",
            params![id.to_string()],
        );

        // Clean up embedding (if vector table exists)
        let _ = conn.execute(
            "DELETE FROM entity_embeddings WHERE entity_id = ?1",
            params![id.to_string()],
        );

        Ok(deleted > 0)
    }

    pub fn list_by_type(
        &self,
        entity_type: EntityType,
        limit: usize,
    ) -> Result<Vec<Entity>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT * FROM entities WHERE entity_type = ?1 AND superseded = 0
                 ORDER BY updated_at DESC LIMIT ?2",
            )
            .map_err(|e| MemoryError::Storage(format!("prepare: {e}")))?;

        let rows = stmt
            .query_map(params![entity_type.as_str(), limit as i64], |row| {
                Ok(row_to_entity(row))
            })
            .map_err(|e| MemoryError::Storage(format!("list: {e}")))?;

        let mut entities = Vec::new();
        for row in rows {
            match row {
                Ok(Ok(entity)) => entities.push(entity),
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(MemoryError::Storage(format!("row: {e}"))),
            }
        }
        Ok(entities)
    }

    /// List all non-superseded entities ranked by confidence: stated > observed > inferred,
    /// then by reinforcement count, then recency. Used for L1 memory generation.
    pub fn list_all_ranked(&self, limit: usize) -> Result<Vec<Entity>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT * FROM entities WHERE superseded = 0
                 ORDER BY
                   CASE confidence_source
                     WHEN 'stated' THEN 3
                     WHEN 'observed' THEN 2
                     WHEN 'inferred' THEN 1
                     ELSE 0
                   END DESC,
                   reinforcement_count DESC,
                   updated_at DESC
                 LIMIT ?1",
            )
            .map_err(|e| MemoryError::Storage(format!("prepare: {e}")))?;

        let rows = stmt
            .query_map(params![limit as i64], |row| Ok(row_to_entity(row)))
            .map_err(|e| MemoryError::Storage(format!("list: {e}")))?;

        let mut entities = Vec::new();
        for row in rows {
            match row {
                Ok(Ok(entity)) => entities.push(entity),
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(MemoryError::Storage(format!("row: {e}"))),
            }
        }
        Ok(entities)
    }

    pub fn count_by_type(&self, entity_type: EntityType) -> Result<usize, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entities WHERE entity_type = ?1 AND superseded = 0",
                params![entity_type.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| MemoryError::Storage(format!("count: {e}")))?;
        Ok(count as usize)
    }

    pub fn count_all(&self) -> Result<usize, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entities WHERE superseded = 0",
                [],
                |row| row.get(0),
            )
            .map_err(|e| MemoryError::Storage(format!("count: {e}")))?;
        Ok(count as usize)
    }

    // === FTS5 Search ===

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Entity>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT e.* FROM entities e
                 JOIN entities_fts fts ON e.rowid = fts.rowid
                 WHERE entities_fts MATCH ?1 AND e.superseded = 0
                 ORDER BY rank
                 LIMIT ?2",
            )
            .map_err(|e| MemoryError::Storage(format!("prepare search: {e}")))?;

        let rows = stmt
            .query_map(params![query, limit as i64], |row| Ok(row_to_entity(row)))
            .map_err(|e| MemoryError::Storage(format!("search: {e}")))?;

        let mut entities = Vec::new();
        for row in rows {
            match row {
                Ok(Ok(entity)) => entities.push(entity),
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(MemoryError::Storage(format!("row: {e}"))),
            }
        }
        Ok(entities)
    }

    pub fn search_by_type(
        &self,
        query: &str,
        entity_type: EntityType,
        limit: usize,
    ) -> Result<Vec<Entity>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT e.* FROM entities e
                 JOIN entities_fts fts ON e.rowid = fts.rowid
                 WHERE entities_fts MATCH ?1 AND e.entity_type = ?2 AND e.superseded = 0
                 ORDER BY rank
                 LIMIT ?3",
            )
            .map_err(|e| MemoryError::Storage(format!("prepare: {e}")))?;

        let rows = stmt
            .query_map(
                params![query, entity_type.as_str(), limit as i64],
                |row| Ok(row_to_entity(row)),
            )
            .map_err(|e| MemoryError::Storage(format!("search: {e}")))?;

        let mut entities = Vec::new();
        for row in rows {
            match row {
                Ok(Ok(entity)) => entities.push(entity),
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(MemoryError::Storage(format!("row: {e}"))),
            }
        }
        Ok(entities)
    }

    // === Relations ===

    pub fn add_relation(
        &self,
        source_id: Uuid,
        relation_type: RelationType,
        target_id: Uuid,
    ) -> Result<(), MemoryError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO relations (source_id, relation_type, target_id, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                source_id.to_string(),
                relation_type.as_str(),
                target_id.to_string(),
                Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|e| MemoryError::Storage(format!("add relation: {e}")))?;
        Ok(())
    }

    pub fn get_relations(&self, entity_id: Uuid) -> Result<Vec<Relation>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let id_str = entity_id.to_string();
        let mut stmt = conn
            .prepare(
                "SELECT source_id, relation_type, target_id, created_at FROM relations
                 WHERE source_id = ?1 OR target_id = ?1",
            )
            .map_err(|e| MemoryError::Storage(format!("prepare: {e}")))?;

        let rows = stmt
            .query_map(params![id_str], |row| {
                let source_str: String = row.get(0)?;
                let rel_str: String = row.get(1)?;
                let target_str: String = row.get(2)?;
                let created_str: String = row.get(3)?;
                Ok((source_str, rel_str, target_str, created_str))
            })
            .map_err(|e| MemoryError::Storage(format!("query: {e}")))?;

        let mut relations = Vec::new();
        for row in rows {
            let (source_str, rel_str, target_str, created_str) =
                row.map_err(|e| MemoryError::Storage(format!("row: {e}")))?;

            if let (Ok(source), Some(rel_type), Ok(target)) = (
                Uuid::parse_str(&source_str),
                RelationType::from_str(&rel_str),
                Uuid::parse_str(&target_str),
            ) {
                relations.push(Relation {
                    source_id: source,
                    relation_type: rel_type,
                    target_id: target,
                    created_at: DateTime::parse_from_rfc3339(&created_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                });
            }
        }
        Ok(relations)
    }

    pub fn get_neighbors(&self, entity_id: Uuid) -> Result<Vec<(Uuid, RelationType)>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let id_str = entity_id.to_string();
        let mut stmt = conn
            .prepare(
                "SELECT target_id, relation_type FROM relations WHERE source_id = ?1
                 UNION
                 SELECT source_id, relation_type FROM relations WHERE target_id = ?1",
            )
            .map_err(|e| MemoryError::Storage(format!("prepare: {e}")))?;

        let rows = stmt
            .query_map(params![id_str], |row| {
                let id_str: String = row.get(0)?;
                let rel_str: String = row.get(1)?;
                Ok((id_str, rel_str))
            })
            .map_err(|e| MemoryError::Storage(format!("query: {e}")))?;

        let mut neighbors = Vec::new();
        for row in rows {
            let (id_str, rel_str) =
                row.map_err(|e| MemoryError::Storage(format!("row: {e}")))?;
            if let (Ok(id), Some(rel)) = (Uuid::parse_str(&id_str), RelationType::from_str(&rel_str)) {
                neighbors.push((id, rel));
            }
        }
        Ok(neighbors)
    }

    pub fn delete_relation(
        &self,
        source_id: Uuid,
        relation_type: RelationType,
        target_id: Uuid,
    ) -> Result<bool, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let deleted = conn
            .execute(
                "DELETE FROM relations WHERE source_id = ?1 AND relation_type = ?2 AND target_id = ?3",
                params![
                    source_id.to_string(),
                    relation_type.as_str(),
                    target_id.to_string(),
                ],
            )
            .map_err(|e| MemoryError::Storage(format!("delete relation: {e}")))?;
        Ok(deleted > 0)
    }

    // === Search-Before-Create ===

    /// Store a fact with search-before-create deduplication.
    /// Searches for existing entities of the same type with similar titles.
    /// Returns Inserted, Reinforced, or Superseded.
    pub fn store_fact(&self, entity: &Entity) -> Result<StoreFactResult, MemoryError> {
        // Search for existing entities of the same type.
        // Quote the title for FTS5 to prevent special character interpretation.
        let fts_query = format!("\"{}\"", entity.title.replace('"', "\"\""));
        let candidates = self.search_by_type(&fts_query, entity.entity_type, 5)?;

        // Check for exact or near-exact title match
        let title_lower = entity.title.to_lowercase();
        for candidate in &candidates {
            let candidate_lower = candidate.title.to_lowercase();

            if candidate_lower == title_lower {
                // Exact match — reinforce
                return self.reinforce_entity(candidate.id);
            }
        }

        // No match — insert new
        self.insert_entity(entity)?;
        Ok(StoreFactResult::Inserted {
            entity_id: entity.id,
        })
    }

    /// Reinforce an existing entity (increment count, update timestamp).
    fn reinforce_entity(&self, entity_id: Uuid) -> Result<StoreFactResult, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE entities SET reinforcement_count = reinforcement_count + 1,
             updated_at = ?2, accessed_at = ?2 WHERE id = ?1",
            params![entity_id.to_string(), now],
        )
        .map_err(|e| MemoryError::Storage(format!("reinforce: {e}")))?;

        let new_count: u32 = conn
            .query_row(
                "SELECT reinforcement_count FROM entities WHERE id = ?1",
                params![entity_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| MemoryError::Storage(format!("get count: {e}")))?;

        debug!(id = %entity_id, count = new_count, "entity reinforced");
        Ok(StoreFactResult::Reinforced {
            entity_id,
            new_count,
        })
    }

    /// Supersede an existing entity with a new one.
    pub fn supersede_entity(
        &self,
        old_id: Uuid,
        new_entity: &Entity,
    ) -> Result<StoreFactResult, MemoryError> {
        // Mark old as superseded
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE entities SET superseded = 1, updated_at = ?2 WHERE id = ?1",
            params![old_id.to_string(), Utc::now().to_rfc3339()],
        )
        .map_err(|e| MemoryError::Storage(format!("supersede: {e}")))?;
        drop(conn);

        // Insert new entity
        self.insert_entity(new_entity)?;

        // Add supersedes relation
        self.add_relation(new_entity.id, RelationType::Supersedes, old_id)?;

        debug!(old = %old_id, new = %new_entity.id, "entity superseded");
        Ok(StoreFactResult::Superseded {
            old_entity_id: old_id,
            new_entity_id: new_entity.id,
        })
    }

    // === Vector Operations ===

    /// Initialize vector storage with the given dimensions.
    /// Must be called after opening the store if embeddings are enabled.
    pub fn init_vectors(&self, dims: usize) -> Result<(), MemoryError> {
        let conn = self.conn.lock().unwrap();
        vector::create_vector_table(&conn, dims)?;
        Ok(())
    }

    /// Store an embedding for an entity.
    pub fn store_embedding(&self, entity_id: Uuid, embedding: &[f32]) -> Result<(), MemoryError> {
        let conn = self.conn.lock().unwrap();
        vector::store_embedding(&conn, entity_id, embedding)
    }

    /// Search for entities similar to a query embedding.
    pub fn search_similar(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<vector::SimilarityResult>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        vector::search_similar(&conn, query_embedding, limit)
    }

    /// Search for entities similar to a query, filtered to a subset.
    pub fn search_similar_filtered(
        &self,
        query_embedding: &[f32],
        entity_ids: &[Uuid],
        limit: usize,
    ) -> Result<Vec<vector::SimilarityResult>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        vector::search_similar_filtered(&conn, query_embedding, entity_ids, limit)
    }

    /// Check if an entity has a stored embedding.
    pub fn has_embedding(&self, entity_id: Uuid) -> Result<bool, MemoryError> {
        let conn = self.conn.lock().unwrap();
        vector::has_embedding(&conn, entity_id)
    }

    /// Count total stored embeddings.
    pub fn count_embeddings(&self) -> Result<usize, MemoryError> {
        let conn = self.conn.lock().unwrap();
        vector::count_embeddings(&conn)
    }

    // === Tags ===

    pub fn search_by_tags(
        &self,
        tags: &[String],
        limit: usize,
    ) -> Result<Vec<Entity>, MemoryError> {
        if tags.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.conn.lock().unwrap();
        // Build JSON containment check for each tag
        let conditions: Vec<String> = tags
            .iter()
            .enumerate()
            .map(|(i, _)| format!("json_each.value = ?{}", i + 2))
            .collect();
        let where_clause = conditions.join(" OR ");

        let sql = format!(
            "SELECT DISTINCT e.* FROM entities e, json_each(e.tags)
             WHERE ({where_clause}) AND e.superseded = 0
             ORDER BY e.updated_at DESC LIMIT ?1"
        );

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| MemoryError::Storage(format!("prepare tags: {e}")))?;

        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        param_values.push(Box::new(limit as i64));
        for tag in tags {
            param_values.push(Box::new(tag.clone()));
        }

        let refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

        let rows = stmt
            .query_map(refs.as_slice(), |row| Ok(row_to_entity(row)))
            .map_err(|e| MemoryError::Storage(format!("tags query: {e}")))?;

        let mut entities = Vec::new();
        for row in rows {
            match row {
                Ok(Ok(entity)) => entities.push(entity),
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(MemoryError::Storage(format!("row: {e}"))),
            }
        }
        Ok(entities)
    }
}

// === Row Mapping ===

fn row_to_entity(row: &rusqlite::Row) -> Result<Entity, MemoryError> {
    let id_str: String = row.get(0).map_err(|e| MemoryError::Storage(format!("id: {e}")))?;
    let type_str: String = row.get(1).map_err(|e| MemoryError::Storage(format!("type: {e}")))?;
    let title: String = row.get(2).map_err(|e| MemoryError::Storage(format!("title: {e}")))?;
    let content: Option<String> = row.get(3).map_err(|e| MemoryError::Storage(format!("content: {e}")))?;
    let conf_str: String = row.get(4).map_err(|e| MemoryError::Storage(format!("conf: {e}")))?;
    let reinforcement: u32 = row.get(5).map_err(|e| MemoryError::Storage(format!("reinf: {e}")))?;
    let superseded: i32 = row.get(6).map_err(|e| MemoryError::Storage(format!("super: {e}")))?;
    let tags_json: String = row.get(7).map_err(|e| MemoryError::Storage(format!("tags: {e}")))?;
    let session_str: Option<String> = row.get(8).map_err(|e| MemoryError::Storage(format!("session: {e}")))?;
    let created_str: String = row.get(9).map_err(|e| MemoryError::Storage(format!("created: {e}")))?;
    let updated_str: String = row.get(10).map_err(|e| MemoryError::Storage(format!("updated: {e}")))?;
    let accessed_str: String = row.get(11).map_err(|e| MemoryError::Storage(format!("accessed: {e}")))?;

    let parse_dt = |s: &str| -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now())
    };

    Ok(Entity {
        id: Uuid::parse_str(&id_str).map_err(|e| MemoryError::Storage(format!("parse id: {e}")))?,
        entity_type: EntityType::from_str(&type_str)
            .ok_or_else(|| MemoryError::Storage(format!("unknown type: {type_str}")))?,
        title,
        content,
        confidence_source: ConfidenceSource::from_str(&conf_str).unwrap_or(ConfidenceSource::Inferred),
        reinforcement_count: reinforcement,
        superseded: superseded != 0,
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        source_session: session_str.and_then(|s| Uuid::parse_str(&s).ok()),
        created_at: parse_dt(&created_str),
        updated_at: parse_dt(&updated_str),
        accessed_at: parse_dt(&accessed_str),
    })
}

/// Extension trait for optional query results.
trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> MemoryStore {
        MemoryStore::in_memory().unwrap()
    }

    #[test]
    fn insert_and_get() {
        let store = test_store();
        let entity = Entity::new(EntityType::Fact, "Rust is fast");
        store.insert_entity(&entity).unwrap();

        let retrieved = store.get_entity(entity.id).unwrap().unwrap();
        assert_eq!(retrieved.title, "Rust is fast");
        assert_eq!(retrieved.entity_type, EntityType::Fact);
    }

    #[test]
    fn get_nonexistent() {
        let store = test_store();
        assert!(store.get_entity(Uuid::new_v4()).unwrap().is_none());
    }

    #[test]
    fn update_entity() {
        let store = test_store();
        let mut entity = Entity::new(EntityType::Fact, "Rust is fast");
        store.insert_entity(&entity).unwrap();

        entity.title = "Rust is very fast".into();
        entity.updated_at = Utc::now();
        store.update_entity(&entity).unwrap();

        let retrieved = store.get_entity(entity.id).unwrap().unwrap();
        assert_eq!(retrieved.title, "Rust is very fast");
    }

    #[test]
    fn delete_entity() {
        let store = test_store();
        let entity = Entity::new(EntityType::Fact, "temporary");
        store.insert_entity(&entity).unwrap();

        assert!(store.delete_entity(entity.id).unwrap());
        assert!(store.get_entity(entity.id).unwrap().is_none());
    }

    #[test]
    fn list_by_type() {
        let store = test_store();
        store.insert_entity(&Entity::new(EntityType::Fact, "fact 1")).unwrap();
        store.insert_entity(&Entity::new(EntityType::Fact, "fact 2")).unwrap();
        store.insert_entity(&Entity::new(EntityType::Decision, "decision 1")).unwrap();

        let facts = store.list_by_type(EntityType::Fact, 10).unwrap();
        assert_eq!(facts.len(), 2);

        let decisions = store.list_by_type(EntityType::Decision, 10).unwrap();
        assert_eq!(decisions.len(), 1);
    }

    #[test]
    fn count_by_type() {
        let store = test_store();
        store.insert_entity(&Entity::new(EntityType::Fact, "f1")).unwrap();
        store.insert_entity(&Entity::new(EntityType::Fact, "f2")).unwrap();
        store.insert_entity(&Entity::new(EntityType::Note, "n1")).unwrap();

        assert_eq!(store.count_by_type(EntityType::Fact).unwrap(), 2);
        assert_eq!(store.count_by_type(EntityType::Note).unwrap(), 1);
        assert_eq!(store.count_all().unwrap(), 3);
    }

    #[test]
    fn fts5_search() {
        let store = test_store();
        store.insert_entity(&Entity::new(EntityType::Fact, "Rust ownership model")).unwrap();
        store.insert_entity(&Entity::new(EntityType::Fact, "Python GIL limitations")).unwrap();
        store
            .insert_entity(&Entity::new(EntityType::Decision, "Use Rust for backend"))
            .unwrap();

        let results = store.search("Rust", 10).unwrap();
        assert_eq!(results.len(), 2);

        let results = store.search("Python", 10).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn fts5_search_by_type() {
        let store = test_store();
        store.insert_entity(&Entity::new(EntityType::Fact, "Rust is fast")).unwrap();
        store
            .insert_entity(&Entity::new(EntityType::Decision, "Use Rust"))
            .unwrap();

        let results = store.search_by_type("Rust", EntityType::Fact, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_type, EntityType::Fact);
    }

    #[test]
    fn relations_crud() {
        let store = test_store();
        let e1 = Entity::new(EntityType::Fact, "fact");
        let e2 = Entity::new(EntityType::Decision, "decision");
        store.insert_entity(&e1).unwrap();
        store.insert_entity(&e2).unwrap();

        store.add_relation(e1.id, RelationType::Supports, e2.id).unwrap();

        let rels = store.get_relations(e1.id).unwrap();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].relation_type, RelationType::Supports);

        let neighbors = store.get_neighbors(e1.id).unwrap();
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].0, e2.id);

        store.delete_relation(e1.id, RelationType::Supports, e2.id).unwrap();
        assert!(store.get_relations(e1.id).unwrap().is_empty());
    }

    #[test]
    fn store_fact_insert() {
        let store = test_store();
        let entity = Entity::new(EntityType::Fact, "User prefers Rust");

        match store.store_fact(&entity).unwrap() {
            StoreFactResult::Inserted { entity_id } => assert_eq!(entity_id, entity.id),
            _ => panic!("expected Inserted"),
        }
    }

    #[test]
    fn store_fact_reinforce() {
        let store = test_store();
        let e1 = Entity::new(EntityType::Fact, "User prefers Rust");
        store.insert_entity(&e1).unwrap();

        // Same title — should reinforce
        let e2 = Entity::new(EntityType::Fact, "User prefers Rust");
        match store.store_fact(&e2).unwrap() {
            StoreFactResult::Reinforced { entity_id, new_count } => {
                assert_eq!(entity_id, e1.id);
                assert_eq!(new_count, 1);
            }
            _ => panic!("expected Reinforced"),
        }
    }

    #[test]
    fn store_fact_reinforce_case_insensitive() {
        let store = test_store();
        let e1 = Entity::new(EntityType::Fact, "User prefers Rust");
        store.insert_entity(&e1).unwrap();

        let e2 = Entity::new(EntityType::Fact, "user prefers rust");
        match store.store_fact(&e2).unwrap() {
            StoreFactResult::Reinforced { entity_id, .. } => {
                assert_eq!(entity_id, e1.id);
            }
            _ => panic!("expected Reinforced"),
        }
    }

    #[test]
    fn supersede_entity() {
        let store = test_store();
        let old = Entity::new(EntityType::Preference, "Prefers Python");
        store.insert_entity(&old).unwrap();

        let new = Entity::new(EntityType::Preference, "Prefers Rust");
        let result = store.supersede_entity(old.id, &new).unwrap();

        match result {
            StoreFactResult::Superseded { old_entity_id, new_entity_id } => {
                assert_eq!(old_entity_id, old.id);
                assert_eq!(new_entity_id, new.id);
            }
            _ => panic!("expected Superseded"),
        }

        // Old should be superseded
        let old_entity = store.get_entity(old.id).unwrap().unwrap();
        assert!(old_entity.superseded);

        // Supersedes relation should exist
        let rels = store.get_relations(new.id).unwrap();
        assert!(rels.iter().any(|r| r.relation_type == RelationType::Supersedes));
    }

    #[test]
    fn tags_on_entity() {
        let store = test_store();
        let entity = Entity::new(EntityType::Fact, "Tagged fact")
            .with_tags(vec!["rust".into(), "performance".into()]);
        store.insert_entity(&entity).unwrap();

        let retrieved = store.get_entity(entity.id).unwrap().unwrap();
        assert_eq!(retrieved.tags, vec!["rust", "performance"]);
    }

    #[test]
    fn search_by_tags() {
        let store = test_store();
        store
            .insert_entity(
                &Entity::new(EntityType::Fact, "Rust perf")
                    .with_tags(vec!["rust".into(), "performance".into()]),
            )
            .unwrap();
        store
            .insert_entity(
                &Entity::new(EntityType::Fact, "Python perf")
                    .with_tags(vec!["python".into(), "performance".into()]),
            )
            .unwrap();

        let results = store.search_by_tags(&["rust".into()], 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust perf");

        let results = store.search_by_tags(&["performance".into()], 10).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn superseded_excluded_from_search() {
        let store = test_store();
        let old = Entity::new(EntityType::Fact, "Old fact about Rust");
        store.insert_entity(&old).unwrap();

        let new = Entity::new(EntityType::Fact, "New fact about Rust");
        store.supersede_entity(old.id, &new).unwrap();

        let results = store.search("Rust", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, new.id);
    }
}
