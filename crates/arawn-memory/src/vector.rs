//! Vector storage and similarity search using sqlite-vec.
//!
//! Provides embedding storage and semantic search via the sqlite-vec
//! SQLite extension (vec0 virtual tables).

use rusqlite::{Connection, params};
use tracing::{debug, info};
use uuid::Uuid;
use zerocopy::IntoBytes;

use crate::error::MemoryError;

/// Initialize sqlite-vec extension globally for all connections.
/// Must be called before opening any memory database that uses vectors.
pub fn init_vector_extension() {
    use rusqlite::ffi::sqlite3_auto_extension;
    use sqlite_vec::sqlite3_vec_init;

    unsafe {
        #[allow(clippy::missing_transmute_annotations)]
        sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
    }
}

/// Check if sqlite-vec extension is loaded. Returns version string.
pub fn check_vector_extension(conn: &Connection) -> Result<String, MemoryError> {
    conn.query_row("SELECT vec_version()", [], |row| row.get(0))
        .map_err(|e| MemoryError::Storage(format!("vec_version: {e}")))
}

/// Create the vector embeddings table with the given dimensions.
pub fn create_vector_table(conn: &Connection, dims: usize) -> Result<(), MemoryError> {
    let sql = format!(
        "CREATE VIRTUAL TABLE IF NOT EXISTS entity_embeddings USING vec0(
            entity_id TEXT PRIMARY KEY,
            embedding float[{dims}]
        )"
    );
    conn.execute_batch(&sql)
        .map_err(|e| MemoryError::Storage(format!("create vector table: {e}")))?;
    info!(dims, "vector table created");
    Ok(())
}

/// Drop the vector embeddings table (for reindex).
pub fn drop_vector_table(conn: &Connection) -> Result<(), MemoryError> {
    conn.execute_batch("DROP TABLE IF EXISTS entity_embeddings")
        .map_err(|e| MemoryError::Storage(format!("drop vector table: {e}")))?;
    Ok(())
}

/// Store an embedding for an entity. Replaces existing if present.
pub fn store_embedding(
    conn: &Connection,
    entity_id: Uuid,
    embedding: &[f32],
) -> Result<(), MemoryError> {
    // vec0 doesn't support INSERT OR REPLACE, so delete first
    let _ = conn.execute(
        "DELETE FROM entity_embeddings WHERE entity_id = ?1",
        params![entity_id.to_string()],
    );

    conn.execute(
        "INSERT INTO entity_embeddings (entity_id, embedding) VALUES (?1, ?2)",
        params![entity_id.to_string(), embedding.as_bytes()],
    )
    .map_err(|e| MemoryError::Storage(format!("store embedding: {e}")))?;

    debug!(entity_id = %entity_id, "embedding stored");
    Ok(())
}

/// Delete an embedding for an entity.
pub fn delete_embedding(conn: &Connection, entity_id: Uuid) -> Result<bool, MemoryError> {
    let rows = conn
        .execute(
            "DELETE FROM entity_embeddings WHERE entity_id = ?1",
            params![entity_id.to_string()],
        )
        .map_err(|e| MemoryError::Storage(format!("delete embedding: {e}")))?;
    Ok(rows > 0)
}

/// Check if an embedding exists for an entity.
pub fn has_embedding(conn: &Connection, entity_id: Uuid) -> Result<bool, MemoryError> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entity_embeddings WHERE entity_id = ?1",
            params![entity_id.to_string()],
            |row| row.get(0),
        )
        .map_err(|e| MemoryError::Storage(format!("has embedding: {e}")))?;
    Ok(count > 0)
}

/// Count total stored embeddings.
pub fn count_embeddings(conn: &Connection) -> Result<usize, MemoryError> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM entity_embeddings", [], |row| {
            row.get(0)
        })
        .map_err(|e| MemoryError::Storage(format!("count embeddings: {e}")))?;
    Ok(count as usize)
}

/// Result of a similarity search.
#[derive(Debug, Clone)]
pub struct SimilarityResult {
    pub entity_id: Uuid,
    /// Distance from query vector (lower = more similar).
    pub distance: f32,
}

/// Search for entities similar to a query embedding.
/// Returns top-k results ordered by distance (ascending).
pub fn search_similar(
    conn: &Connection,
    query_embedding: &[f32],
    limit: usize,
) -> Result<Vec<SimilarityResult>, MemoryError> {
    let mut stmt = conn
        .prepare(
            "SELECT entity_id, distance
             FROM entity_embeddings
             WHERE embedding MATCH ?1
             ORDER BY distance
             LIMIT ?2",
        )
        .map_err(|e| MemoryError::Storage(format!("prepare search: {e}")))?;

    let mut rows = stmt
        .query(params![query_embedding.as_bytes(), limit as i64])
        .map_err(|e| MemoryError::Storage(format!("search: {e}")))?;

    let mut results = Vec::new();
    while let Some(row) = rows.next().map_err(|e| MemoryError::Storage(format!("row: {e}")))? {
        let id_str: String = row.get(0).map_err(|e| MemoryError::Storage(format!("id: {e}")))?;
        let distance: f32 = row.get(1).map_err(|e| MemoryError::Storage(format!("dist: {e}")))?;

        if let Ok(entity_id) = Uuid::parse_str(&id_str) {
            results.push(SimilarityResult {
                entity_id,
                distance,
            });
        }
    }

    debug!(count = results.len(), limit, "similarity search complete");
    Ok(results)
}

/// Search for entities similar to a query, filtered to a subset of entity IDs.
pub fn search_similar_filtered(
    conn: &Connection,
    query_embedding: &[f32],
    entity_ids: &[Uuid],
    limit: usize,
) -> Result<Vec<SimilarityResult>, MemoryError> {
    if entity_ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders: Vec<String> = (0..entity_ids.len())
        .map(|i| format!("?{}", i + 3))
        .collect();
    let in_clause = placeholders.join(", ");

    let sql = format!(
        "SELECT entity_id, distance
         FROM entity_embeddings
         WHERE embedding MATCH ?1
           AND entity_id IN ({in_clause})
         ORDER BY distance
         LIMIT ?2"
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| MemoryError::Storage(format!("prepare filtered: {e}")))?;

    let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = vec![
        Box::new(query_embedding.as_bytes().to_vec()),
        Box::new(limit as i64),
    ];
    for id in entity_ids {
        param_values.push(Box::new(id.to_string()));
    }
    let refs: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|b| b.as_ref()).collect();

    let mut rows = stmt
        .query(refs.as_slice())
        .map_err(|e| MemoryError::Storage(format!("filtered search: {e}")))?;

    let mut results = Vec::new();
    while let Some(row) = rows.next().map_err(|e| MemoryError::Storage(format!("row: {e}")))? {
        let id_str: String = row.get(0).map_err(|e| MemoryError::Storage(format!("id: {e}")))?;
        let distance: f32 = row.get(1).map_err(|e| MemoryError::Storage(format!("dist: {e}")))?;

        if let Ok(entity_id) = Uuid::parse_str(&id_str) {
            results.push(SimilarityResult {
                entity_id,
                distance,
            });
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        init_vector_extension();
        let conn = Connection::open_in_memory().unwrap();
        create_vector_table(&conn, 4).unwrap();
        conn
    }

    #[test]
    fn extension_loads() {
        init_vector_extension();
        let conn = Connection::open_in_memory().unwrap();
        let version = check_vector_extension(&conn).unwrap();
        assert!(!version.is_empty());
    }

    #[test]
    fn store_and_check() {
        let conn = test_conn();
        let id = Uuid::new_v4();
        store_embedding(&conn, id, &[0.1, 0.2, 0.3, 0.4]).unwrap();
        assert!(has_embedding(&conn, id).unwrap());
        assert_eq!(count_embeddings(&conn).unwrap(), 1);
    }

    #[test]
    fn delete_embedding_works() {
        let conn = test_conn();
        let id = Uuid::new_v4();
        store_embedding(&conn, id, &[0.1, 0.2, 0.3, 0.4]).unwrap();
        assert!(delete_embedding(&conn, id).unwrap());
        assert!(!has_embedding(&conn, id).unwrap());
    }

    #[test]
    fn similarity_search() {
        let conn = test_conn();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        store_embedding(&conn, id1, &[1.0, 0.0, 0.0, 0.0]).unwrap();
        store_embedding(&conn, id2, &[0.9, 0.1, 0.0, 0.0]).unwrap();
        store_embedding(&conn, id3, &[0.0, 0.0, 1.0, 0.0]).unwrap();

        let results = search_similar(&conn, &[1.0, 0.0, 0.0, 0.0], 10).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].entity_id, id1); // exact match first
        assert!(results[0].distance < 0.01);
    }

    #[test]
    fn similarity_search_with_limit() {
        let conn = test_conn();
        for _ in 0..5 {
            store_embedding(&conn, Uuid::new_v4(), &[1.0, 0.0, 0.0, 0.0]).unwrap();
        }
        let results = search_similar(&conn, &[1.0, 0.0, 0.0, 0.0], 2).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn update_embedding() {
        let conn = test_conn();
        let id = Uuid::new_v4();
        store_embedding(&conn, id, &[1.0, 0.0, 0.0, 0.0]).unwrap();
        store_embedding(&conn, id, &[0.0, 1.0, 0.0, 0.0]).unwrap();

        assert_eq!(count_embeddings(&conn).unwrap(), 1);
        let results = search_similar(&conn, &[0.0, 1.0, 0.0, 0.0], 1).unwrap();
        assert_eq!(results[0].entity_id, id);
        assert!(results[0].distance < 0.01);
    }

    #[test]
    fn filtered_search() {
        let conn = test_conn();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        store_embedding(&conn, id1, &[1.0, 0.0, 0.0, 0.0]).unwrap();
        store_embedding(&conn, id2, &[0.9, 0.1, 0.0, 0.0]).unwrap();
        store_embedding(&conn, id3, &[0.0, 0.0, 1.0, 0.0]).unwrap();

        let results =
            search_similar_filtered(&conn, &[1.0, 0.0, 0.0, 0.0], &[id1, id3], 10).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].entity_id, id1);
    }

    #[test]
    fn filtered_search_empty() {
        let conn = test_conn();
        let results = search_similar_filtered(&conn, &[1.0, 0.0, 0.0, 0.0], &[], 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_empty_table() {
        let conn = test_conn();
        let results = search_similar(&conn, &[1.0, 0.0, 0.0, 0.0], 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn delete_nonexistent() {
        let conn = test_conn();
        assert!(!delete_embedding(&conn, Uuid::new_v4()).unwrap());
    }

    #[test]
    fn drop_and_recreate() {
        let conn = test_conn();
        store_embedding(&conn, Uuid::new_v4(), &[1.0, 0.0, 0.0, 0.0]).unwrap();
        drop_vector_table(&conn).unwrap();
        create_vector_table(&conn, 4).unwrap();
        assert_eq!(count_embeddings(&conn).unwrap(), 0);
    }
}
