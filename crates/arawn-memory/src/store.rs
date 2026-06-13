//! Knowledge base store: graphqlite (Cypher / EAV) for entities + relations,
//! plus a colocated FTS5 virtual table and (optional) vector extension table
//! on the same sqlite handle.
//!
//! The graphqlite-backed node graph is the sole source of truth for entity
//! and relation records. FTS5 + vector tables are derived projections kept in
//! sync via explicit Rust dual-writes inside a single sqlite transaction.

use std::path::Path;
use std::sync::Mutex;

use chrono::{DateTime, NaiveDate, Utc};
use graphqlite::Connection as GraphConnection;
use rusqlite::params;
use tracing::{debug, info};
use uuid::Uuid;

use crate::cypher_schema::{
    entity_label, entity_to_props, node_to_entity, relation_type_from_str, relation_type_str,
};
use crate::error::MemoryError;
use crate::person_profile::{PersonProfile, RelationToUser};
use crate::types::*;
use crate::vector;

/// Knowledge base store.
///
/// Holds a single `graphqlite::Connection`. Cypher operations run via
/// `.cypher_builder(...)`; raw SQL for the FTS5 index, vector table, and
/// transaction control runs via `.sqlite_connection()`.
pub struct MemoryStore {
    conn: Mutex<GraphConnection>,
}

impl MemoryStore {
    /// Open or create a memory database at the given path.
    pub fn open(path: &Path) -> Result<Self, MemoryError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| MemoryError::Storage(format!("create dir: {e}")))?;
        }

        let conn = GraphConnection::open(path)
            .map_err(|e| MemoryError::Storage(format!("open db: {e}")))?;

        conn.sqlite_connection()
            .execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
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
        let conn = GraphConnection::open_in_memory()
            .map_err(|e| MemoryError::Storage(format!("open in-memory: {e}")))?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<(), MemoryError> {
        let conn = self.conn.lock().unwrap();
        let sql = conn.sqlite_connection();

        // Drop the T-0239 legacy schema (pre-graphqlite world). These
        // tables shouldn't exist on any current db; the drops are
        // defensive and idempotent for that one-time transition.
        //
        // CRITICAL: do NOT drop `entities_fts` here. `MemoryStore::open`
        // runs on every lens KB load — dropping the FTS table on
        // reopen wipes the index for every entity ever extracted. The
        // CREATE below is `IF NOT EXISTS` precisely so a re-open is a
        // no-op when the schema is already current. (Bug landed during
        // the original T-0239 graphqlite migration; UATs surfaced it
        // post-seed when the server's reopen wiped seed FTS rows.)
        sql.execute_batch(
            "DROP TRIGGER IF EXISTS entities_ai;
             DROP TRIGGER IF EXISTS entities_ad;
             DROP TRIGGER IF EXISTS entities_au;
             DROP TABLE IF EXISTS entities;
             DROP TABLE IF EXISTS relations;",
        )
        .map_err(|e| MemoryError::Storage(format!("drop legacy: {e}")))?;

        // Standalone FTS5 keyed on entity_id (uuid string). No external-content
        // linkage — entity records live in graphqlite EAV, not a parent table.
        sql.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS entities_fts USING fts5(
                entity_id UNINDEXED,
                title,
                content,
                tokenize = 'unicode61'
            );",
        )
        .map_err(|e| MemoryError::Storage(format!("fts5 migrate: {e}")))?;

        // ARAWN-I-0064 T-B: person_profile sidecar for structured Person
        // fields. `entity_id` is an app-level FK into the graphqlite
        // Entity model (no SQLite FK because Entity lives in EAV).
        // Cascade-delete handled in `delete_entity`.
        sql.execute_batch(
            "CREATE TABLE IF NOT EXISTS person_profile (
                entity_id            TEXT PRIMARY KEY NOT NULL,
                role                 TEXT,
                relation_to_user     TEXT,
                reports_to_person_id TEXT,
                hire_date            TEXT,
                last_1on1            TEXT,
                pronouns             TEXT,
                time_zone            TEXT,
                growth_areas         TEXT NOT NULL DEFAULT '[]',
                current_concerns     TEXT NOT NULL DEFAULT '[]',
                created_at           TEXT NOT NULL,
                updated_at           TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_person_profile_relation_to_user
                ON person_profile(relation_to_user)
                WHERE relation_to_user IS NOT NULL;
            CREATE INDEX IF NOT EXISTS idx_person_profile_reports_to
                ON person_profile(reports_to_person_id)
                WHERE reports_to_person_id IS NOT NULL;",
        )
        .map_err(|e| MemoryError::Storage(format!("person_profile migrate: {e}")))?;

        Ok(())
    }

    // === Entity CRUD ===
    //
    // Each write runs Cypher + FTS5 (and the optional vector cleanup on
    // delete) inside one sqlite transaction. The Cypher executor uses the
    // same `rusqlite::Connection` under the hood, so a BEGIN/COMMIT around
    // the whole flow gives us atomicity across both APIs.

    pub fn insert_entity(&self, entity: &Entity) -> Result<(), MemoryError> {
        let conn = self.conn.lock().unwrap();
        insert_entity_locked(&conn, entity)
    }

    pub fn get_entity(&self, id: Uuid) -> Result<Option<Entity>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        fetch_entity_by_id(&conn, id)
    }

    pub fn update_entity(&self, entity: &Entity) -> Result<(), MemoryError> {
        let conn = self.conn.lock().unwrap();
        with_tx(&conn, |conn| {
            cypher_upsert_entity(conn, entity)?;
            fts_upsert(conn.sqlite_connection(), entity)?;
            Ok(())
        })?;
        Ok(())
    }

    pub fn delete_entity(&self, id: Uuid) -> Result<bool, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let id_str = id.to_string();

        // Check existence before deleting so the bool return is meaningful.
        let existed = cypher_entity_exists(&conn, &id_str)?;
        if !existed {
            return Ok(false);
        }

        with_tx(&conn, |conn| {
            conn.cypher_builder("MATCH (n {id: $id}) DETACH DELETE n")
                .param("id", id_str.clone())
                .run()
                .map_err(|e| MemoryError::Storage(format!("cypher delete entity: {e}")))?;

            let sql = conn.sqlite_connection();
            sql.execute(
                "DELETE FROM entities_fts WHERE entity_id = ?1",
                params![id_str.clone()],
            )
            .map_err(|e| MemoryError::Storage(format!("fts delete: {e}")))?;

            // Vector table is created lazily — ignore if absent.
            let _ = sql.execute(
                "DELETE FROM entity_embeddings WHERE entity_id = ?1",
                params![id_str.clone()],
            );

            // ARAWN-I-0064 T-B: cascade-delete the person_profile sidecar
            // so removing a Person entity doesn't leave an orphan profile
            // row. Safe to run for non-Person entities — no row, no-op.
            sql.execute(
                "DELETE FROM person_profile WHERE entity_id = ?1",
                params![id_str.clone()],
            )
            .map_err(|e| MemoryError::Storage(format!("person_profile cascade delete: {e}")))?;
            Ok(())
        })?;
        Ok(true)
    }

    pub fn list_by_type(
        &self,
        entity_type: EntityType,
        limit: usize,
    ) -> Result<Vec<Entity>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let label = entity_label(entity_type);
        let query = format!(
            "MATCH (n:{label}) WHERE n.superseded = false RETURN n ORDER BY n.updated_at DESC LIMIT $lim"
        );
        let result = conn
            .cypher_builder(&query)
            .param("lim", limit as i64)
            .run()
            .map_err(|e| MemoryError::Storage(format!("cypher list_by_type: {e}")))?;
        rows_to_entities(&result)
    }

    /// List all non-superseded entities ranked by confidence: stated > observed > inferred,
    /// then by reinforcement count, then recency. Used for L1 memory generation.
    ///
    /// Ranking is computed in Rust — graphqlite's Cypher dialect doesn't accept
    /// `CASE` expressions, and the entity set is small enough that pulling
    /// candidates and sorting client-side is cheaper than designing an
    /// equivalent Cypher predicate.
    pub fn list_all_ranked(&self, limit: usize) -> Result<Vec<Entity>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let result = conn
            .cypher("MATCH (n) WHERE n.superseded = false RETURN n")
            .map_err(|e| MemoryError::Storage(format!("cypher list_all_ranked: {e}")))?;
        let mut entities = rows_to_entities(&result)?;
        entities.sort_by(|a, b| {
            let rank = |c: ConfidenceSource| match c {
                ConfidenceSource::Stated => 3,
                ConfidenceSource::Observed => 2,
                ConfidenceSource::Inferred => 1,
            };
            rank(b.confidence_source)
                .cmp(&rank(a.confidence_source))
                .then(b.reinforcement_count.cmp(&a.reinforcement_count))
                .then(b.updated_at.cmp(&a.updated_at))
        });
        entities.truncate(limit);
        Ok(entities)
    }

    pub fn count_by_type(&self, entity_type: EntityType) -> Result<usize, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let label = entity_label(entity_type);
        let query = format!("MATCH (n:{label}) WHERE n.superseded = false RETURN count(n) AS cnt");
        let result = conn
            .cypher(&query)
            .map_err(|e| MemoryError::Storage(format!("cypher count_by_type: {e}")))?;
        let cnt: i64 = if result.is_empty() {
            0
        } else {
            result[0].get("cnt").unwrap_or(0)
        };
        Ok(cnt.max(0) as usize)
    }

    pub fn count_all(&self) -> Result<usize, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let result = conn
            .cypher("MATCH (n) WHERE n.superseded = false RETURN count(n) AS cnt")
            .map_err(|e| MemoryError::Storage(format!("cypher count_all: {e}")))?;
        let cnt: i64 = if result.is_empty() {
            0
        } else {
            result[0].get("cnt").unwrap_or(0)
        };
        Ok(cnt.max(0) as usize)
    }

    // === FTS5 Search ===
    //
    // FTS5 returns entity_ids ranked by relevance; we fetch full entities
    // via Cypher. Superseded filter happens at the Cypher fetch — we
    // over-fetch FTS by 2× to compensate for drops.

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Entity>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let ids = fts_search(conn.sqlite_connection(), query, None, limit * 2)?;
        let mut out = Vec::with_capacity(limit);
        for id in ids {
            if let Some(e) = fetch_entity_by_id(&conn, id)?
                && !e.superseded
            {
                out.push(e);
                if out.len() == limit {
                    break;
                }
            }
        }
        Ok(out)
    }

    /// Like [`search`] but returns each hit's FTS relevance score (higher is
    /// a better match; it's the negated bm25 value, so ≥ 0 for any match).
    /// Backs the extractor's link-by-name confidence floor (ARAWN-T-0482) —
    /// a weak FTS hit can be rejected rather than silently linked.
    pub fn search_scored(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(Entity, f64)>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let scored = fts_search_scored(conn.sqlite_connection(), query, limit * 2)?;
        let mut out = Vec::with_capacity(limit);
        for (id, score) in scored {
            if let Some(e) = fetch_entity_by_id(&conn, id)?
                && !e.superseded
            {
                out.push((e, score));
                if out.len() == limit {
                    break;
                }
            }
        }
        Ok(out)
    }

    pub fn search_by_type(
        &self,
        query: &str,
        entity_type: EntityType,
        limit: usize,
    ) -> Result<Vec<Entity>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        search_by_type_locked(&conn, query, entity_type, limit)
    }

    // === Relations ===

    pub fn add_relation(
        &self,
        source_id: Uuid,
        relation_type: RelationType,
        target_id: Uuid,
    ) -> Result<(), MemoryError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        cypher_upsert_relation(&conn, source_id, relation_type, target_id, &now)
    }

    pub fn get_relations(&self, entity_id: Uuid) -> Result<Vec<Relation>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let out = conn
            .cypher_builder(
                "MATCH (a {id: $id})-[r]->(b) RETURN a.id AS src, type(r) AS rt, b.id AS tgt, r.created_at AS ts",
            )
            .param("id", entity_id.to_string())
            .run()
            .map_err(|e| MemoryError::Storage(format!("cypher get_relations out: {e}")))?;
        let inc = conn
            .cypher_builder(
                "MATCH (a)-[r]->(b {id: $id}) RETURN a.id AS src, type(r) AS rt, b.id AS tgt, r.created_at AS ts",
            )
            .param("id", entity_id.to_string())
            .run()
            .map_err(|e| MemoryError::Storage(format!("cypher get_relations in: {e}")))?;

        let mut relations = Vec::new();
        for row in out.iter().chain(inc.iter()) {
            let src: String = row.get("src").unwrap_or_default();
            let rt: String = row.get("rt").unwrap_or_default();
            let tgt: String = row.get("tgt").unwrap_or_default();
            let ts: String = row.get("ts").unwrap_or_default();
            if let (Ok(s), Some(r), Ok(t)) = (
                Uuid::parse_str(&src),
                relation_type_from_str(&rt),
                Uuid::parse_str(&tgt),
            ) {
                relations.push(Relation {
                    source_id: s,
                    relation_type: r,
                    target_id: t,
                    created_at: DateTime::parse_from_rfc3339(&ts)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                });
            }
        }
        Ok(relations)
    }

    pub fn get_neighbors(&self, entity_id: Uuid) -> Result<Vec<(Uuid, RelationType)>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let result = conn
            .cypher_builder(
                "MATCH (n {id: $id})-[r]-(m) RETURN DISTINCT m.id AS mid, type(r) AS rt",
            )
            .param("id", entity_id.to_string())
            .run()
            .map_err(|e| MemoryError::Storage(format!("cypher get_neighbors: {e}")))?;
        let mut neighbors = Vec::new();
        for row in result.iter() {
            let mid: String = row.get("mid").unwrap_or_default();
            let rt: String = row.get("rt").unwrap_or_default();
            if let (Ok(id), Some(rel)) = (Uuid::parse_str(&mid), relation_type_from_str(&rt)) {
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
        let rt = relation_type_str(relation_type);
        let exists_query =
            format!("MATCH (a {{id: $src}})-[r:{rt}]->(b {{id: $tgt}}) RETURN count(r) AS cnt");
        let exists = conn
            .cypher_builder(&exists_query)
            .param("src", source_id.to_string())
            .param("tgt", target_id.to_string())
            .run()
            .map_err(|e| MemoryError::Storage(format!("cypher edge exists: {e}")))?;
        let cnt: i64 = if exists.is_empty() {
            0
        } else {
            exists[0].get("cnt").unwrap_or(0)
        };
        if cnt == 0 {
            return Ok(false);
        }

        let delete_query = format!("MATCH (a {{id: $src}})-[r:{rt}]->(b {{id: $tgt}}) DELETE r");
        conn.cypher_builder(&delete_query)
            .param("src", source_id.to_string())
            .param("tgt", target_id.to_string())
            .run()
            .map_err(|e| MemoryError::Storage(format!("cypher delete relation: {e}")))?;
        Ok(true)
    }

    // === Search-Before-Create ===

    /// Store a fact with search-before-create deduplication.
    /// Searches for existing entities of the same type via FTS5; an exact
    /// (case-insensitive) title match reinforces the existing entity.
    pub fn store_fact(&self, entity: &Entity) -> Result<StoreFactResult, MemoryError> {
        // ARAWN-T-0481 (P2-6): hold the connection lock across the entire
        // dedupe-or-insert. Previously `search_by_type`, `reinforce_entity`
        // and `insert_entity` each acquired and released the lock
        // independently, so a concurrent delete could slip between the search
        // and a reinforce (losing the fact), or a concurrent insert of the
        // same title could land between two searches (a duplicate). One lock
        // around the whole compound op closes that window.
        let conn = self.conn.lock().unwrap();

        // Quote the title for FTS5 to prevent special character interpretation.
        let fts_query = format!("\"{}\"", entity.title.replace('"', "\"\""));
        let candidates = search_by_type_locked(&conn, &fts_query, entity.entity_type, 5)?;

        let title_lower = entity.title.to_lowercase();
        for candidate in &candidates {
            if candidate.title.to_lowercase() == title_lower {
                return reinforce_entity_locked(&conn, candidate.id);
            }
        }

        insert_entity_locked(&conn, entity)?;
        Ok(StoreFactResult::Inserted {
            entity_id: entity.id,
        })
    }

    /// Reinforce an existing entity (increment count, refresh timestamps).
    #[allow(dead_code)]
    fn reinforce_entity(&self, entity_id: Uuid) -> Result<StoreFactResult, MemoryError> {
        let conn = self.conn.lock().unwrap();
        reinforce_entity_locked(&conn, entity_id)
    }

    /// Supersede an existing entity with a new one.
    pub fn supersede_entity(
        &self,
        old_id: Uuid,
        new_entity: &Entity,
    ) -> Result<StoreFactResult, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.cypher_builder("MATCH (n {id: $id}) SET n.superseded = true, n.updated_at = $now")
            .param("id", old_id.to_string())
            .param("now", now)
            .run()
            .map_err(|e| MemoryError::Storage(format!("cypher supersede: {e}")))?;
        drop(conn);

        self.insert_entity(new_entity)?;
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
        vector::create_vector_table(conn.sqlite_connection(), dims)?;
        Ok(())
    }

    /// Store an embedding for an entity.
    pub fn store_embedding(&self, entity_id: Uuid, embedding: &[f32]) -> Result<(), MemoryError> {
        let conn = self.conn.lock().unwrap();
        vector::store_embedding(conn.sqlite_connection(), entity_id, embedding)
    }

    /// Search for entities similar to a query embedding.
    pub fn search_similar(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<vector::SimilarityResult>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        vector::search_similar(conn.sqlite_connection(), query_embedding, limit)
    }

    /// Search for entities similar to a query, filtered to a subset.
    pub fn search_similar_filtered(
        &self,
        query_embedding: &[f32],
        entity_ids: &[Uuid],
        limit: usize,
    ) -> Result<Vec<vector::SimilarityResult>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        vector::search_similar_filtered(
            conn.sqlite_connection(),
            query_embedding,
            entity_ids,
            limit,
        )
    }

    /// Check if an entity has a stored embedding.
    pub fn has_embedding(&self, entity_id: Uuid) -> Result<bool, MemoryError> {
        let conn = self.conn.lock().unwrap();
        vector::has_embedding(conn.sqlite_connection(), entity_id)
    }

    /// Count total stored embeddings.
    pub fn count_embeddings(&self) -> Result<usize, MemoryError> {
        let conn = self.conn.lock().unwrap();
        vector::count_embeddings(conn.sqlite_connection())
    }

    // === Tags ===

    /// Tag search loads all non-superseded entities and filters in Rust.
    /// Tags are stored as a JSON-string property; native Cypher matching
    /// against JSON strings isn't expressible in graphqlite's dialect.
    /// Adequate for memory-scale corpora; revisit if hot.
    pub fn search_by_tags(
        &self,
        tags: &[String],
        limit: usize,
    ) -> Result<Vec<Entity>, MemoryError> {
        if tags.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn.lock().unwrap();
        let result = conn
            .cypher("MATCH (n) WHERE n.superseded = false RETURN n")
            .map_err(|e| MemoryError::Storage(format!("cypher search_by_tags: {e}")))?;
        let mut entities = rows_to_entities(&result)?;
        entities.retain(|e| e.tags.iter().any(|t| tags.contains(t)));
        entities.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        entities.truncate(limit);
        Ok(entities)
    }

    // === Person profile sidecar (ARAWN-I-0064 T-B) ===
    //
    // Structured Person fields live in a dedicated SQLite table; the
    // Entity itself stays a closed-enum graphqlite node. Profile rows
    // are optional — a Person can exist without one (callers fall back
    // to title/content). `delete_entity` cascades into this table to
    // avoid orphans.

    /// Insert-or-update a person_profile row. Updates `updated_at` to
    /// `profile.updated_at` on conflict; caller is responsible for
    /// stamping it.
    pub fn upsert_person_profile(&self, profile: &PersonProfile) -> Result<(), MemoryError> {
        let conn = self.conn.lock().unwrap();
        let sql = conn.sqlite_connection();
        let growth_json = serde_json::to_string(&profile.growth_areas)
            .map_err(|e| MemoryError::Storage(format!("serialize growth_areas: {e}")))?;
        let concerns_json = serde_json::to_string(&profile.current_concerns)
            .map_err(|e| MemoryError::Storage(format!("serialize current_concerns: {e}")))?;
        sql.execute(
            "INSERT INTO person_profile (
                entity_id, role, relation_to_user, reports_to_person_id,
                hire_date, last_1on1, pronouns, time_zone,
                growth_areas, current_concerns, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4,
                ?5, ?6, ?7, ?8,
                ?9, ?10, ?11, ?12
             )
             ON CONFLICT(entity_id) DO UPDATE SET
                role                 = excluded.role,
                relation_to_user     = excluded.relation_to_user,
                reports_to_person_id = excluded.reports_to_person_id,
                hire_date            = excluded.hire_date,
                last_1on1            = excluded.last_1on1,
                pronouns             = excluded.pronouns,
                time_zone            = excluded.time_zone,
                growth_areas         = excluded.growth_areas,
                current_concerns     = excluded.current_concerns,
                updated_at           = excluded.updated_at",
            params![
                profile.entity_id.to_string(),
                profile.role,
                profile.relation_to_user.map(|r| r.as_str().to_string()),
                profile.reports_to_person_id.map(|u| u.to_string()),
                profile.hire_date.map(|d| d.format("%Y-%m-%d").to_string()),
                profile.last_1on1.map(|d| d.to_rfc3339()),
                profile.pronouns,
                profile.time_zone,
                growth_json,
                concerns_json,
                profile.created_at.to_rfc3339(),
                profile.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| MemoryError::Storage(format!("upsert person_profile: {e}")))?;
        Ok(())
    }

    pub fn get_person_profile(
        &self,
        entity_id: Uuid,
    ) -> Result<Option<PersonProfile>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let sql = conn.sqlite_connection();
        let mut stmt = sql
            .prepare(
                "SELECT entity_id, role, relation_to_user, reports_to_person_id,
                        hire_date, last_1on1, pronouns, time_zone,
                        growth_areas, current_concerns, created_at, updated_at
                 FROM person_profile
                 WHERE entity_id = ?1",
            )
            .map_err(|e| MemoryError::Storage(format!("prepare get_person_profile: {e}")))?;
        let row = stmt.query_row(params![entity_id.to_string()], parse_person_profile_row);
        match row {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(MemoryError::Storage(format!("get person_profile: {e}"))),
        }
    }

    pub fn delete_person_profile(&self, entity_id: Uuid) -> Result<bool, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let sql = conn.sqlite_connection();
        let n = sql
            .execute(
                "DELETE FROM person_profile WHERE entity_id = ?1",
                params![entity_id.to_string()],
            )
            .map_err(|e| MemoryError::Storage(format!("delete person_profile: {e}")))?;
        Ok(n > 0)
    }

    /// All profiles with the given `relation_to_user` (e.g. `Manages` →
    /// the user's direct reports). Used by the L0 memory stack render
    /// in T-D. Ordered most-recently-updated first.
    pub fn list_person_profiles_by_relation_to_user(
        &self,
        rel: RelationToUser,
    ) -> Result<Vec<PersonProfile>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let sql = conn.sqlite_connection();
        let mut stmt = sql
            .prepare(
                "SELECT entity_id, role, relation_to_user, reports_to_person_id,
                        hire_date, last_1on1, pronouns, time_zone,
                        growth_areas, current_concerns, created_at, updated_at
                 FROM person_profile
                 WHERE relation_to_user = ?1
                 ORDER BY updated_at DESC",
            )
            .map_err(|e| MemoryError::Storage(format!("prepare list_by_relation_to_user: {e}")))?;
        let rows = stmt
            .query_map(params![rel.as_str()], parse_person_profile_row)
            .map_err(|e| MemoryError::Storage(format!("query list_by_relation_to_user: {e}")))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| MemoryError::Storage(format!("row list_by_relation: {e}")))?);
        }
        Ok(out)
    }
}

/// Shared row → PersonProfile parser. Column order must match the SELECT
/// list in both `get_person_profile` and
/// `list_person_profiles_by_relation_to_user`.
fn parse_person_profile_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PersonProfile> {
    let entity_id_str: String = row.get(0)?;
    let entity_id = Uuid::parse_str(&entity_id_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })?;
    let role: Option<String> = row.get(1)?;
    let relation_to_user_str: Option<String> = row.get(2)?;
    let reports_to_str: Option<String> = row.get(3)?;
    let hire_date_str: Option<String> = row.get(4)?;
    let last_1on1_str: Option<String> = row.get(5)?;
    let pronouns: Option<String> = row.get(6)?;
    let time_zone: Option<String> = row.get(7)?;
    let growth_json: String = row.get(8)?;
    let concerns_json: String = row.get(9)?;
    let created_at_str: String = row.get(10)?;
    let updated_at_str: String = row.get(11)?;

    let now = Utc::now();
    Ok(PersonProfile {
        entity_id,
        role,
        relation_to_user: relation_to_user_str
            .as_deref()
            .and_then(RelationToUser::from_str),
        reports_to_person_id: reports_to_str
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok()),
        hire_date: hire_date_str
            .as_deref()
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
        last_1on1: last_1on1_str
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc)),
        pronouns,
        time_zone,
        growth_areas: serde_json::from_str(&growth_json).unwrap_or_default(),
        current_concerns: serde_json::from_str(&concerns_json).unwrap_or_default(),
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or(now),
        updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or(now),
    })
}

// === Cypher helpers ===

/// Search-by-type over an already-held connection (ARAWN-T-0481). Factored
/// out of the public `search_by_type` so `store_fact` can run it under the
/// same lock as the subsequent reinforce/insert.
fn search_by_type_locked(
    conn: &GraphConnection,
    query: &str,
    entity_type: EntityType,
    limit: usize,
) -> Result<Vec<Entity>, MemoryError> {
    let ids = fts_search(conn.sqlite_connection(), query, None, limit * 4)?;
    let mut out = Vec::with_capacity(limit);
    for id in ids {
        if let Some(e) = fetch_entity_by_id(conn, id)?
            && !e.superseded
            && e.entity_type == entity_type
        {
            out.push(e);
            if out.len() == limit {
                break;
            }
        }
    }
    Ok(out)
}

/// Insert an entity over an already-held connection (ARAWN-T-0481).
fn insert_entity_locked(conn: &GraphConnection, entity: &Entity) -> Result<(), MemoryError> {
    with_tx(conn, |conn| {
        cypher_upsert_entity(conn, entity)?;
        fts_upsert(conn.sqlite_connection(), entity)?;
        Ok(())
    })?;
    debug!(id = %entity.id, title = %entity.title, "entity inserted");
    Ok(())
}

/// Reinforce an entity over an already-held connection (ARAWN-T-0481). The
/// read + write run under the caller's single lock, so concurrent
/// reinforcements can't interleave and undercount. (graphqlite's Cypher has
/// no arithmetic SET against a property reference, so the count is
/// round-tripped rather than incremented in SQL — the lock is what makes it
/// atomic.)
fn reinforce_entity_locked(
    conn: &GraphConnection,
    entity_id: Uuid,
) -> Result<StoreFactResult, MemoryError> {
    let now = Utc::now().to_rfc3339();
    let id_str = entity_id.to_string();
    let row = conn
        .cypher_builder("MATCH (n {id: $id}) RETURN n.reinforcement_count AS cnt")
        .param("id", id_str.clone())
        .run()
        .map_err(|e| MemoryError::Storage(format!("cypher read count: {e}")))?;
    let current: i64 = if row.is_empty() {
        return Err(MemoryError::Storage(format!(
            "reinforce: entity {entity_id} not found"
        )));
    } else {
        row[0].get("cnt").unwrap_or(0)
    };
    let new_count = (current + 1).max(0) as u32;
    conn.cypher_builder(
        "MATCH (n {id: $id}) \
         SET n.reinforcement_count = $cnt, n.updated_at = $now, n.accessed_at = $now",
    )
    .param("id", id_str)
    .param("cnt", new_count as i64)
    .param("now", now)
    .run()
    .map_err(|e| MemoryError::Storage(format!("cypher reinforce: {e}")))?;
    debug!(id = %entity_id, count = new_count, "entity reinforced");
    Ok(StoreFactResult::Reinforced {
        entity_id,
        new_count,
    })
}

/// Run `body` inside a sqlite transaction on the shared connection. Cypher
/// queries issued via the same `GraphConnection` are part of the same sqlite
/// session, so a single BEGIN/COMMIT pair envelops both APIs.
fn with_tx<F>(conn: &GraphConnection, body: F) -> Result<(), MemoryError>
where
    F: FnOnce(&GraphConnection) -> Result<(), MemoryError>,
{
    let sql = conn.sqlite_connection();
    sql.execute_batch("BEGIN")
        .map_err(|e| MemoryError::Storage(format!("begin tx: {e}")))?;
    match body(conn) {
        Ok(()) => sql
            .execute_batch("COMMIT")
            .map_err(|e| MemoryError::Storage(format!("commit tx: {e}"))),
        Err(e) => {
            let _ = sql.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

fn cypher_entity_exists(conn: &GraphConnection, id: &str) -> Result<bool, MemoryError> {
    let res = conn
        .cypher_builder("MATCH (n {id: $id}) RETURN count(n) AS cnt")
        .param("id", id)
        .run()
        .map_err(|e| MemoryError::Storage(format!("cypher exists: {e}")))?;
    let cnt: i64 = if res.is_empty() {
        0
    } else {
        res[0].get("cnt").unwrap_or(0)
    };
    Ok(cnt > 0)
}

fn fetch_entity_by_id(conn: &GraphConnection, id: Uuid) -> Result<Option<Entity>, MemoryError> {
    let res = conn
        .cypher_builder("MATCH (n {id: $id}) RETURN n LIMIT 1")
        .param("id", id.to_string())
        .run()
        .map_err(|e| MemoryError::Storage(format!("cypher get entity: {e}")))?;
    if res.is_empty() {
        return Ok(None);
    }
    let node = res[0]
        .get_value("n")
        .ok_or_else(|| MemoryError::Storage("missing column 'n'".into()))?;
    Ok(Some(node_to_entity(node)?))
}

/// MERGE-style upsert: create node-with-label if absent, otherwise SET every
/// scalar from `entity_to_props`. We emulate `MERGE` explicitly because
/// `MERGE … SET n += $props` is not part of graphqlite's Cypher dialect.
fn cypher_upsert_entity(conn: &GraphConnection, entity: &Entity) -> Result<(), MemoryError> {
    let label = entity_label(entity.entity_type);
    let id = entity.id.to_string();
    let props = entity_to_props(entity);

    if cypher_entity_exists(conn, &id)? {
        let query = "MATCH (n {id: $id}) \
             SET n.title = $title, n.content = $content, \
                 n.confidence_source = $confidence_source, \
                 n.reinforcement_count = $reinforcement_count, \
                 n.superseded = $superseded, n.tags = $tags, \
                 n.tags_ontology = $tags_ontology, \
                 n.source_session = $source_session, \
                 n.created_at = $created_at, n.updated_at = $updated_at, \
                 n.accessed_at = $accessed_at";
        conn.cypher_builder(query)
            .params(&props)
            .run()
            .map_err(|e| MemoryError::Storage(format!("cypher SET: {e}")))?;
    } else {
        let query = format!(
            "CREATE (n:{label} {{id: $id, title: $title, content: $content, \
             confidence_source: $confidence_source, \
             reinforcement_count: $reinforcement_count, \
             superseded: $superseded, tags: $tags, \
             tags_ontology: $tags_ontology, \
             source_session: $source_session, \
             created_at: $created_at, updated_at: $updated_at, \
             accessed_at: $accessed_at}})"
        );
        conn.cypher_builder(&query)
            .params(&props)
            .run()
            .map_err(|e| MemoryError::Storage(format!("cypher CREATE: {e}")))?;
    }
    Ok(())
}

/// MERGE-style edge upsert. Edge type is from the closed `RelationType` enum,
/// so inlining it into the Cypher template is safe (no user data).
fn cypher_upsert_relation(
    conn: &GraphConnection,
    source_id: Uuid,
    relation_type: RelationType,
    target_id: Uuid,
    created_at: &str,
) -> Result<(), MemoryError> {
    let rt = relation_type_str(relation_type);
    let exists_query =
        format!("MATCH (a {{id: $src}})-[r:{rt}]->(b {{id: $tgt}}) RETURN count(r) AS cnt");
    let exists = conn
        .cypher_builder(&exists_query)
        .param("src", source_id.to_string())
        .param("tgt", target_id.to_string())
        .run()
        .map_err(|e| MemoryError::Storage(format!("cypher edge exists: {e}")))?;
    let cnt: i64 = if exists.is_empty() {
        0
    } else {
        exists[0].get("cnt").unwrap_or(0)
    };
    if cnt > 0 {
        return Ok(());
    }
    let create_query = format!(
        "MATCH (a {{id: $src}}), (b {{id: $tgt}}) \
         CREATE (a)-[:{rt} {{created_at: $ts}}]->(b)"
    );
    conn.cypher_builder(&create_query)
        .param("src", source_id.to_string())
        .param("tgt", target_id.to_string())
        .param("ts", created_at.to_string())
        .run()
        .map_err(|e| MemoryError::Storage(format!("cypher create edge: {e}")))?;
    Ok(())
}

/// Map a `MATCH … RETURN n` result set into `Vec<Entity>`.
fn rows_to_entities(result: &graphqlite::CypherResult) -> Result<Vec<Entity>, MemoryError> {
    let mut entities = Vec::with_capacity(result.len());
    for row in result.iter() {
        if let Some(node) = row.get_value("n") {
            entities.push(node_to_entity(node)?);
        }
    }
    Ok(entities)
}

// === FTS5 helpers ===

/// Upsert the FTS row for an entity. FTS5 has no MERGE; emulate via
/// DELETE-then-INSERT (idempotent under the same `entity_id`).
///
/// The FTS5 virtual table has two text columns (title, content). To
/// give `signal_search` recall across both tag fields without adding
/// columns (and a schema migration), we fold each tag list into the
/// indexed `content` blob. Filtering by tag remains an exact-match
/// operation against the entity's tag arrays — FTS is recall-only.
fn fts_upsert(sql: &rusqlite::Connection, entity: &Entity) -> Result<(), MemoryError> {
    let id = entity.id.to_string();
    sql.execute(
        "DELETE FROM entities_fts WHERE entity_id = ?1",
        params![id.clone()],
    )
    .map_err(|e| MemoryError::Storage(format!("fts delete-before-insert: {e}")))?;
    let mut content_text = entity.content.clone().unwrap_or_default();
    if !entity.tags.is_empty() {
        if !content_text.is_empty() {
            content_text.push('\n');
        }
        content_text.push_str(&entity.tags.join(" "));
    }
    if !entity.tags_ontology.is_empty() {
        if !content_text.is_empty() {
            content_text.push('\n');
        }
        content_text.push_str(&entity.tags_ontology.join(" "));
    }
    sql.execute(
        "INSERT INTO entities_fts (entity_id, title, content) VALUES (?1, ?2, ?3)",
        params![id, entity.title, content_text],
    )
    .map_err(|e| MemoryError::Storage(format!("fts insert: {e}")))?;
    Ok(())
}

/// FTS5 text search returning ranked entity_ids. Caller does any
/// superseded/type filtering after fetching the entities via Cypher.
///
/// `query` is the raw FTS5 MATCH expression; callers that want literal-text
/// matching should pre-quote (`"hello world"`).
/// FTS search returning each hit's relevance score (ARAWN-T-0482). The score
/// is `-bm25(...)` so higher = better and any match is ≥ 0; callers can apply
/// a confidence floor against it. Ordered best-first.
fn fts_search_scored(
    sql: &rusqlite::Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<(Uuid, f64)>, MemoryError> {
    let mut stmt = sql
        .prepare(
            "SELECT entity_id, bm25(entities_fts) AS score FROM entities_fts \
             WHERE entities_fts MATCH ?1 ORDER BY rank LIMIT ?2",
        )
        .map_err(|e| MemoryError::Storage(format!("prepare fts scored: {e}")))?;
    let rows = stmt
        .query_map(params![query, limit as i64], |row| {
            let id_str: String = row.get(0)?;
            let bm25: f64 = row.get(1)?;
            Ok((id_str, bm25))
        })
        .map_err(|e| MemoryError::Storage(format!("fts scored: {e}")))?;
    let mut out = Vec::new();
    for row in rows {
        let (id_str, bm25) = row.map_err(|e| MemoryError::Storage(format!("row: {e}")))?;
        if let Ok(id) = Uuid::parse_str(&id_str) {
            // Negate bm25 so higher = better and a confidence floor reads naturally.
            out.push((id, -bm25));
        }
    }
    Ok(out)
}

fn fts_search(
    sql: &rusqlite::Connection,
    query: &str,
    _scope: Option<()>,
    limit: usize,
) -> Result<Vec<Uuid>, MemoryError> {
    let mut stmt = sql
        .prepare(
            "SELECT entity_id FROM entities_fts WHERE entities_fts MATCH ?1 ORDER BY rank LIMIT ?2",
        )
        .map_err(|e| MemoryError::Storage(format!("prepare fts search: {e}")))?;
    let rows = stmt
        .query_map(params![query, limit as i64], |row| {
            let id_str: String = row.get(0)?;
            Ok(id_str)
        })
        .map_err(|e| MemoryError::Storage(format!("fts search: {e}")))?;
    let mut ids = Vec::new();
    for row in rows {
        let id_str = row.map_err(|e| MemoryError::Storage(format!("row: {e}")))?;
        if let Ok(id) = Uuid::parse_str(&id_str) {
            ids.push(id);
        }
    }
    Ok(ids)
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

        // FTS reflects the new title.
        let found = store.search("very fast", 5).unwrap();
        assert!(found.iter().any(|e| e.id == entity.id));
    }

    #[test]
    fn delete_entity() {
        let store = test_store();
        let entity = Entity::new(EntityType::Fact, "temporary");
        store.insert_entity(&entity).unwrap();

        assert!(store.delete_entity(entity.id).unwrap());
        assert!(store.get_entity(entity.id).unwrap().is_none());
        // FTS row also gone.
        assert!(store.search("temporary", 5).unwrap().is_empty());
        // Second delete returns false.
        assert!(!store.delete_entity(entity.id).unwrap());
    }

    #[test]
    fn list_by_type() {
        let store = test_store();
        store
            .insert_entity(&Entity::new(EntityType::Fact, "fact 1"))
            .unwrap();
        store
            .insert_entity(&Entity::new(EntityType::Fact, "fact 2"))
            .unwrap();
        store
            .insert_entity(&Entity::new(EntityType::Decision, "decision 1"))
            .unwrap();

        let facts = store.list_by_type(EntityType::Fact, 10).unwrap();
        assert_eq!(facts.len(), 2);

        let decisions = store.list_by_type(EntityType::Decision, 10).unwrap();
        assert_eq!(decisions.len(), 1);
    }

    #[test]
    fn count_by_type() {
        let store = test_store();
        store
            .insert_entity(&Entity::new(EntityType::Fact, "f1"))
            .unwrap();
        store
            .insert_entity(&Entity::new(EntityType::Fact, "f2"))
            .unwrap();
        store
            .insert_entity(&Entity::new(EntityType::Note, "n1"))
            .unwrap();

        assert_eq!(store.count_by_type(EntityType::Fact).unwrap(), 2);
        assert_eq!(store.count_by_type(EntityType::Note).unwrap(), 1);
        assert_eq!(store.count_all().unwrap(), 3);
    }

    #[test]
    fn fts5_search() {
        let store = test_store();
        store
            .insert_entity(&Entity::new(EntityType::Fact, "Rust ownership model"))
            .unwrap();
        store
            .insert_entity(&Entity::new(EntityType::Fact, "Python GIL limitations"))
            .unwrap();
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
        store
            .insert_entity(&Entity::new(EntityType::Fact, "Rust is fast"))
            .unwrap();
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

        store
            .add_relation(e1.id, RelationType::Supports, e2.id)
            .unwrap();

        let rels = store.get_relations(e1.id).unwrap();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].relation_type, RelationType::Supports);

        let neighbors = store.get_neighbors(e1.id).unwrap();
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].0, e2.id);

        store
            .delete_relation(e1.id, RelationType::Supports, e2.id)
            .unwrap();
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

        let e2 = Entity::new(EntityType::Fact, "User prefers Rust");
        match store.store_fact(&e2).unwrap() {
            StoreFactResult::Reinforced {
                entity_id,
                new_count,
            } => {
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
            StoreFactResult::Superseded {
                old_entity_id,
                new_entity_id,
            } => {
                assert_eq!(old_entity_id, old.id);
                assert_eq!(new_entity_id, new.id);
            }
            _ => panic!("expected Superseded"),
        }

        let old_entity = store.get_entity(old.id).unwrap().unwrap();
        assert!(old_entity.superseded);

        let rels = store.get_relations(new.id).unwrap();
        assert!(
            rels.iter()
                .any(|r| r.relation_type == RelationType::Supersedes)
        );
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

    #[test]
    fn fts_row_present_after_insert_and_gone_after_delete() {
        // Proves FTS dual-write is inside the same transaction as the Cypher
        // write — a search hits the row immediately after insert, and after
        // delete the row is gone from both backends.
        let store = test_store();
        let entity = Entity::new(EntityType::Fact, "Goldfinch sighting");
        store.insert_entity(&entity).unwrap();

        let res = store.search("Goldfinch", 5).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].id, entity.id);

        store.delete_entity(entity.id).unwrap();
        assert!(store.search("Goldfinch", 5).unwrap().is_empty());
    }

    // === Person profile sidecar (ARAWN-I-0064 T-B) ===

    #[test]
    fn person_profile_upsert_then_get_roundtrips_all_fields() {
        let store = test_store();
        let person = Entity::new(EntityType::Person, "Sarah Lee");
        store.insert_entity(&person).unwrap();

        let manager = Entity::new(EntityType::Person, "David Chen");
        store.insert_entity(&manager).unwrap();

        let profile = PersonProfile::new(person.id)
            .with_role("Senior EM")
            .with_relation_to_user(RelationToUser::Manages)
            .with_reports_to(manager.id);
        store.upsert_person_profile(&profile).unwrap();

        let fetched = store.get_person_profile(person.id).unwrap().unwrap();
        assert_eq!(fetched.entity_id, person.id);
        assert_eq!(fetched.role.as_deref(), Some("Senior EM"));
        assert_eq!(fetched.relation_to_user, Some(RelationToUser::Manages));
        assert_eq!(fetched.reports_to_person_id, Some(manager.id));
        assert!(fetched.growth_areas.is_empty());
    }

    #[test]
    fn person_profile_upsert_updates_existing_row() {
        let store = test_store();
        let person = Entity::new(EntityType::Person, "Marcus");
        store.insert_entity(&person).unwrap();

        store
            .upsert_person_profile(&PersonProfile::new(person.id).with_role("Staff Eng"))
            .unwrap();
        store
            .upsert_person_profile(
                &PersonProfile::new(person.id)
                    .with_role("Principal Eng")
                    .with_relation_to_user(RelationToUser::Manages),
            )
            .unwrap();

        let p = store.get_person_profile(person.id).unwrap().unwrap();
        assert_eq!(p.role.as_deref(), Some("Principal Eng"));
        assert_eq!(p.relation_to_user, Some(RelationToUser::Manages));
    }

    #[test]
    fn person_profile_get_returns_none_when_absent() {
        let store = test_store();
        assert!(store.get_person_profile(Uuid::new_v4()).unwrap().is_none());
    }

    #[test]
    fn person_profile_delete_returns_false_when_missing() {
        let store = test_store();
        assert!(!store.delete_person_profile(Uuid::new_v4()).unwrap());
    }

    #[test]
    fn list_person_profiles_by_relation_to_user_filters_correctly() {
        let store = test_store();

        let direct1 = Entity::new(EntityType::Person, "Sarah");
        let direct2 = Entity::new(EntityType::Person, "Marcus");
        let manager = Entity::new(EntityType::Person, "David");
        let peer = Entity::new(EntityType::Person, "Anita");
        for e in [&direct1, &direct2, &manager, &peer] {
            store.insert_entity(e).unwrap();
        }
        store
            .upsert_person_profile(
                &PersonProfile::new(direct1.id).with_relation_to_user(RelationToUser::Manages),
            )
            .unwrap();
        store
            .upsert_person_profile(
                &PersonProfile::new(direct2.id).with_relation_to_user(RelationToUser::Manages),
            )
            .unwrap();
        store
            .upsert_person_profile(
                &PersonProfile::new(manager.id)
                    .with_relation_to_user(RelationToUser::ReportsToUser),
            )
            .unwrap();
        store
            .upsert_person_profile(
                &PersonProfile::new(peer.id).with_relation_to_user(RelationToUser::PeerOfUser),
            )
            .unwrap();

        let directs = store
            .list_person_profiles_by_relation_to_user(RelationToUser::Manages)
            .unwrap();
        assert_eq!(directs.len(), 2);
        let direct_ids: std::collections::HashSet<_> =
            directs.iter().map(|p| p.entity_id).collect();
        assert!(direct_ids.contains(&direct1.id));
        assert!(direct_ids.contains(&direct2.id));

        let bosses = store
            .list_person_profiles_by_relation_to_user(RelationToUser::ReportsToUser)
            .unwrap();
        assert_eq!(bosses.len(), 1);
        assert_eq!(bosses[0].entity_id, manager.id);

        let peers = store
            .list_person_profiles_by_relation_to_user(RelationToUser::PeerOfUser)
            .unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].entity_id, peer.id);
    }

    #[test]
    fn delete_entity_cascades_to_person_profile() {
        let store = test_store();
        let person = Entity::new(EntityType::Person, "Cascade Test");
        store.insert_entity(&person).unwrap();
        store
            .upsert_person_profile(&PersonProfile::new(person.id).with_role("Eng"))
            .unwrap();
        assert!(store.get_person_profile(person.id).unwrap().is_some());

        store.delete_entity(person.id).unwrap();

        // Entity gone from graph
        assert!(store.get_entity(person.id).unwrap().is_none());
        // Profile gone too — cascade worked
        assert!(store.get_person_profile(person.id).unwrap().is_none());
    }

    #[test]
    fn person_profile_migration_is_idempotent_on_reopen() {
        // Open then re-open the same path. Migration must be idempotent —
        // a second open mustn't drop or corrupt the table.
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("memory.db");
        let store1 = MemoryStore::open(&db_path).unwrap();
        let person = Entity::new(EntityType::Person, "Survives Reopen");
        store1.insert_entity(&person).unwrap();
        store1
            .upsert_person_profile(&PersonProfile::new(person.id).with_role("CTO"))
            .unwrap();
        drop(store1);

        let store2 = MemoryStore::open(&db_path).unwrap();
        let p = store2.get_person_profile(person.id).unwrap().unwrap();
        assert_eq!(p.role.as_deref(), Some("CTO"));
    }
}
