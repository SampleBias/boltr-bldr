//! Local artifact store with SQLite-based indexing
//!
//! Provides persistent storage and fast querying of ingested, normalized,
//! and emitted artifacts.

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};

use crate::error::{Error, Result};
use crate::models::artifact::*;

fn parse_sqlite_datetime(s: String) -> chrono::DateTime<chrono::Utc> {
    chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
        .map(|dt| dt.and_utc())
        .or_else(|_| chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.to_utc()))
        .unwrap_or_else(|_| chrono::Utc::now())
}

/// The local artifact store backed by SQLite
pub struct Store {
    conn: Connection,
    #[allow(dead_code)]
    data_dir: PathBuf,
}

pub struct ArtifactIndex<'a> {
    pub package_id: &'a str,
    pub file_path: &'a str,
    pub file_type: &'a str,
    pub sha256: &'a str,
    pub size_bytes: u64,
    pub source_db: &'a str,
    pub source_id: &'a str,
}

impl Store {
    /// Open (or create) the store at the given directory
    pub fn open(data_dir: impl Into<PathBuf>) -> Result<Self> {
        let data_dir = data_dir.into();
        std::fs::create_dir_all(&data_dir)?;

        let db_path = data_dir.join("boltr_index.db");
        let conn = Connection::open(&db_path)?;

        let mut store = Self { conn, data_dir };
        store.initialize()?;
        Ok(store)
    }

    /// Open an in-memory store (for testing)
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let mut store = Self {
            conn,
            data_dir: PathBuf::from(":memory:"),
        };
        store.initialize()?;
        Ok(store)
    }

    /// Initialize database schema
    fn initialize(&mut self) -> Result<()> {
        self.conn
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS artifacts (
                    row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    package_id TEXT NOT NULL,
                    file_path TEXT NOT NULL,
                    file_type TEXT NOT NULL,
                    sha256 TEXT NOT NULL,
                    size_bytes INTEGER NOT NULL,
                    indexed_at TEXT NOT NULL DEFAULT (datetime('now')),
                    source_db TEXT NOT NULL,
                    source_id TEXT NOT NULL,
                    UNIQUE(file_path)
                );

                CREATE INDEX IF NOT EXISTS idx_artifacts_package_id ON artifacts(package_id);
                CREATE INDEX IF NOT EXISTS idx_artifacts_source ON artifacts(source_db, source_id);
                CREATE INDEX IF NOT EXISTS idx_artifacts_file_type ON artifacts(file_type);

                CREATE TABLE IF NOT EXISTS pipeline_state (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    source_db TEXT NOT NULL,
                    source_id TEXT NOT NULL,
                    status TEXT NOT NULL,
                    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
                    message TEXT,
                    UNIQUE(source_db, source_id)
                );

                CREATE INDEX IF NOT EXISTS idx_pipeline_status ON pipeline_state(status);
                ",
            )
            .map_err(|e| Error::Store(format!("Schema init failed: {}", e)))?;

        Ok(())
    }

    /// Index a single artifact file
    pub fn index_artifact(&self, artifact: ArtifactIndex<'_>) -> Result<i64> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO artifacts (package_id, file_path, file_type, sha256, size_bytes, source_db, source_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    artifact.package_id,
                    artifact.file_path,
                    artifact.file_type,
                    artifact.sha256,
                    artifact.size_bytes as i64,
                    artifact.source_db,
                    artifact.source_id
                ],
            )
            .map_err(|e| Error::Store(format!("Insert artifact failed: {}", e)))?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Index all files from a manifest
    pub fn index_manifest(&self, manifest: &Manifest, pkg_dir: &Path) -> Result<Vec<i64>> {
        let mut row_ids = Vec::new();

        for file in &manifest.files {
            // Determine source info from manifest sources
            let (source_db, source_id) = manifest
                .sources
                .first()
                .map(|s| (s.database.as_str(), s.id.as_str()))
                .unwrap_or(("unknown", "unknown"));

            let full_path = pkg_dir.join(&file.path);
            let path_str = full_path.to_string_lossy();

            let row_id = self.index_artifact(ArtifactIndex {
                package_id: &manifest.package_id,
                file_path: &path_str,
                file_type: &file.file_type,
                sha256: &file.sha256,
                size_bytes: file.size_bytes,
                source_db,
                source_id,
            })?;

            row_ids.push(row_id);
        }

        tracing::info!(
            package_id = %manifest.package_id,
            count = row_ids.len(),
            "Manifest indexed"
        );

        Ok(row_ids)
    }

    /// Query indexed artifacts by source database and ID
    pub fn find_by_source(&self, source_db: &str, source_id: &str) -> Result<Vec<IndexedArtifact>> {
        let mut stmt = self.conn.prepare(
            "SELECT row_id, package_id, file_path, file_type, sha256, size_bytes, indexed_at, source_db, source_id
             FROM artifacts WHERE source_db = ?1 AND source_id = ?2"
        ).map_err(|e| Error::Store(format!("Query failed: {}", e)))?;

        let rows = stmt
            .query_map(params![source_db, source_id], |row| {
                Ok(IndexedArtifact {
                    row_id: row.get(0)?,
                    package_id: row.get(1)?,
                    file_path: row.get(2)?,
                    file_type: row.get(3)?,
                    sha256: row.get(4)?,
                    size_bytes: row.get(5)?,
                    indexed_at: parse_sqlite_datetime(row.get(6)?),
                    source_db: row.get(7)?,
                    source_id: row.get(8)?,
                })
            })
            .map_err(|e| Error::Store(format!("Query failed: {}", e)))?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::Store(format!("Row parse failed: {}", e)))
    }

    /// Query all artifacts of a given type
    pub fn find_by_type(&self, file_type: &str) -> Result<Vec<IndexedArtifact>> {
        let mut stmt = self.conn.prepare(
            "SELECT row_id, package_id, file_path, file_type, sha256, size_bytes, indexed_at, source_db, source_id
             FROM artifacts WHERE file_type = ?1"
        ).map_err(|e| Error::Store(format!("Query failed: {}", e)))?;

        let rows = stmt
            .query_map(params![file_type], |row| {
                Ok(IndexedArtifact {
                    row_id: row.get(0)?,
                    package_id: row.get(1)?,
                    file_path: row.get(2)?,
                    file_type: row.get(3)?,
                    sha256: row.get(4)?,
                    size_bytes: row.get(5)?,
                    indexed_at: parse_sqlite_datetime(row.get(6)?),
                    source_db: row.get(7)?,
                    source_id: row.get(8)?,
                })
            })
            .map_err(|e| Error::Store(format!("Query failed: {}", e)))?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::Store(format!("Row parse failed: {}", e)))
    }

    /// List all indexed artifacts
    pub fn list_all(&self) -> Result<Vec<IndexedArtifact>> {
        let mut stmt = self.conn.prepare(
            "SELECT row_id, package_id, file_path, file_type, sha256, size_bytes, indexed_at, source_db, source_id
             FROM artifacts ORDER BY indexed_at DESC"
        ).map_err(|e| Error::Store(format!("Query failed: {}", e)))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(IndexedArtifact {
                    row_id: row.get(0)?,
                    package_id: row.get(1)?,
                    file_path: row.get(2)?,
                    file_type: row.get(3)?,
                    sha256: row.get(4)?,
                    size_bytes: row.get(5)?,
                    indexed_at: parse_sqlite_datetime(row.get(6)?),
                    source_db: row.get(7)?,
                    source_id: row.get(8)?,
                })
            })
            .map_err(|e| Error::Store(format!("Query failed: {}", e)))?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::Store(format!("Row parse failed: {}", e)))
    }

    /// List a bounded page of indexed artifacts.
    pub fn list_page(&self, limit: usize, offset: usize) -> Result<Vec<IndexedArtifact>> {
        let limit = limit.clamp(1, 500);
        let mut stmt = self.conn.prepare(
            "SELECT row_id, package_id, file_path, file_type, sha256, size_bytes, indexed_at, source_db, source_id
             FROM artifacts ORDER BY indexed_at DESC, row_id DESC LIMIT ?1 OFFSET ?2"
        ).map_err(|e| Error::Store(format!("Query failed: {}", e)))?;

        let rows = stmt
            .query_map(params![limit as i64, offset as i64], |row| {
                Ok(IndexedArtifact {
                    row_id: row.get(0)?,
                    package_id: row.get(1)?,
                    file_path: row.get(2)?,
                    file_type: row.get(3)?,
                    sha256: row.get(4)?,
                    size_bytes: row.get(5)?,
                    indexed_at: parse_sqlite_datetime(row.get(6)?),
                    source_db: row.get(7)?,
                    source_id: row.get(8)?,
                })
            })
            .map_err(|e| Error::Store(format!("Query failed: {}", e)))?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::Store(format!("Row parse failed: {}", e)))
    }

    /// Update pipeline state for a source
    pub fn update_pipeline_state(
        &self,
        source_db: &str,
        source_id: &str,
        status: &str,
        message: Option<&str>,
    ) -> Result<()> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO pipeline_state (source_db, source_id, status, message, timestamp)
                 VALUES (?1, ?2, ?3, ?4, datetime('now'))",
                params![source_db, source_id, status, message],
            )
            .map_err(|e| Error::Store(format!("Pipeline state update failed: {}", e)))?;

        Ok(())
    }

    /// Get current pipeline state for a source
    pub fn get_pipeline_state(
        &self,
        source_db: &str,
        source_id: &str,
    ) -> Result<Option<(String, String, Option<String>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT status, timestamp, message FROM pipeline_state WHERE source_db = ?1 AND source_id = ?2"
        ).map_err(|e| Error::Store(format!("Query failed: {}", e)))?;

        let result = stmt
            .query_row(params![source_db, source_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .ok();

        Ok(result)
    }

    /// Get the count of indexed artifacts
    pub fn count(&self) -> Result<u64> {
        let count: u64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM artifacts", [], |row| row.get(0))
            .map_err(|e| Error::Store(format!("Count failed: {}", e)))?;

        Ok(count)
    }

    /// Get statistics about the store
    pub fn stats(&self) -> Result<StoreStats> {
        let (total_artifacts, total_yaml, total_npz, total_packages, total_size) = self
            .conn
            .query_row(
                "SELECT
                    COUNT(*),
                    COALESCE(SUM(CASE WHEN file_type = 'yaml' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN file_type = 'npz' THEN 1 ELSE 0 END), 0),
                    COUNT(DISTINCT package_id),
                    COALESCE(SUM(size_bytes), 0)
                 FROM artifacts",
                [],
                |row| {
                    let total_size: i64 = row.get(4)?;
                    Ok((
                        row.get::<_, u64>(0)?,
                        row.get::<_, u64>(1)?,
                        row.get::<_, u64>(2)?,
                        row.get::<_, u64>(3)?,
                        total_size as u64,
                    ))
                },
            )
            .map_err(|e| Error::Store(format!("Stats query failed: {}", e)))?;

        Ok(StoreStats {
            total_artifacts,
            total_yaml,
            total_npz,
            total_packages,
            total_size_bytes: total_size,
        })
    }

    /// Rebuild the index from scratch
    pub fn rebuild_index(&self) -> Result<()> {
        self.conn
            .execute("DELETE FROM artifacts", [])
            .map_err(|e| Error::Store(format!("Clear failed: {}", e)))?;

        tracing::info!("Index cleared for rebuild");
        Ok(())
    }
}

/// Store statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct StoreStats {
    pub total_artifacts: u64,
    pub total_yaml: u64,
    pub total_npz: u64,
    pub total_packages: u64,
    pub total_size_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_open_in_memory() {
        let store = Store::open_in_memory();
        assert!(store.is_ok());
    }

    #[test]
    fn test_store_index_and_query() {
        let store = Store::open_in_memory().unwrap();

        store
            .index_artifact(ArtifactIndex {
                package_id: "pkg1",
                file_path: "/tmp/test.yaml",
                file_type: "yaml",
                sha256: "abc123",
                size_bytes: 1024,
                source_db: "pdb",
                source_id: "1abc",
            })
            .unwrap();

        let results = store.find_by_source("pdb", "1abc").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].file_type, "yaml");
    }

    #[test]
    fn test_store_stats() {
        let store = Store::open_in_memory().unwrap();

        store
            .index_artifact(ArtifactIndex {
                package_id: "pkg1",
                file_path: "/tmp/test.yaml",
                file_type: "yaml",
                sha256: "abc123",
                size_bytes: 1024,
                source_db: "pdb",
                source_id: "1abc",
            })
            .unwrap();
        store
            .index_artifact(ArtifactIndex {
                package_id: "pkg1",
                file_path: "/tmp/data.npz",
                file_type: "npz",
                sha256: "def456",
                size_bytes: 2048,
                source_db: "pdb",
                source_id: "1abc",
            })
            .unwrap();

        let stats = store.stats().unwrap();
        assert_eq!(stats.total_artifacts, 2);
        assert_eq!(stats.total_yaml, 1);
        assert_eq!(stats.total_npz, 1);
        assert_eq!(stats.total_packages, 1);
        assert_eq!(stats.total_size_bytes, 3072);
    }
}
