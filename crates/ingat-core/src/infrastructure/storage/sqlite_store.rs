//! Concurrent SQLite-backed [`VectorStore`] adapter.
//!
//! Multiple OS processes may open the same database: SQLite serialises writers
//! through WAL and a `busy_timeout`. A single mutex-protected connection is
//! used per instance; no pool, ANN index or cache is involved. Embeddings are
//! never computed while a transaction is open.

use std::path::{Path, PathBuf};
use std::time::Duration;

use parking_lot::Mutex;
use rusqlite::{params, Connection, Row};
use uuid::Uuid;

use crate::{
    application::services::VectorStore,
    domain::{
        ContextEmbedding, ContextRecord, ContextSummary, DomainError, MemoryScope, QueryFilters,
    },
};

use super::similarity::cosine_similarity;

/// Minimum SQLite version (3.51.3) required to get the WAL-reset fix.
pub const SQLITE_REQUIRED_VERSION: i32 = 3_051_003;
/// On-disk schema version written to `PRAGMA user_version`.
pub const SQLITE_SCHEMA_VERSION: i32 = 1;
/// Busy-owner wait before SQLite returns `SQLITE_BUSY`.
pub const SQLITE_BUSY_TIMEOUT_MS: u32 = 5_000;

const CREATE_SCHEMA: &str = "\
CREATE TABLE IF NOT EXISTS contexts (
    id          TEXT PRIMARY KEY NOT NULL,
    project     TEXT NOT NULL,
    kind        TEXT NOT NULL,
    scope       TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    model       TEXT NOT NULL,
    dimensions  INTEGER NOT NULL,
    record_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_contexts_project_created ON contexts(project, created_at);
CREATE INDEX IF NOT EXISTS idx_contexts_project_kind ON contexts(project, kind);
";

fn storage_err(err: impl std::fmt::Display) -> DomainError {
    DomainError::storage(err.to_string())
}

fn scope_str(scope: MemoryScope) -> &'static str {
    match scope {
        MemoryScope::Team => "team",
        MemoryScope::Personal => "personal",
    }
}

fn timestamp(created_at: chrono::DateTime<chrono::Utc>) -> String {
    created_at.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
}

/// Embedded vector store backed by SQLite (rusqlite bundled).
pub struct SqliteVectorStore {
    conn: Mutex<Connection>,
    _path: PathBuf,
}

impl SqliteVectorStore {
    /// Opens (or creates) the database at `path`, asserting the bundled SQLite
    /// version, WAL mode, `synchronous = FULL`, the busy timeout and the schema
    /// version.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DomainError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(storage_err)?;
            }
        }

        let actual = rusqlite::version_number();
        if actual < SQLITE_REQUIRED_VERSION {
            return Err(DomainError::storage(format!(
                "bundled SQLite {} is older than required {} (WAL-reset fix)",
                rusqlite::version(),
                SQLITE_REQUIRED_VERSION
            )));
        }

        let conn = Connection::open(&path).map_err(storage_err)?;
        conn.busy_timeout(Duration::from_millis(SQLITE_BUSY_TIMEOUT_MS as u64))
            .map_err(storage_err)?;

        let journal: String = conn
            .query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))
            .map_err(storage_err)?;
        if !journal.eq_ignore_ascii_case("wal") {
            return Err(DomainError::storage(format!(
                "failed to enable WAL mode (got `{journal}`)"
            )));
        }
        conn.execute_batch("PRAGMA synchronous = FULL;")
            .map_err(storage_err)?;

        let schema_version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(storage_err)?;
        if schema_version > SQLITE_SCHEMA_VERSION {
            return Err(DomainError::storage(format!(
                "unsupported schema version {schema_version}; this build supports up to {SQLITE_SCHEMA_VERSION}"
            )));
        }

        conn.execute_batch(CREATE_SCHEMA).map_err(storage_err)?;

        if schema_version < SQLITE_SCHEMA_VERSION {
            conn.pragma_update(None, "user_version", SQLITE_SCHEMA_VERSION)
                .map_err(storage_err)?;
        }

        let store = Self {
            conn: Mutex::new(conn),
            _path: path,
        };
        // Fail loudly on a corrupt database rather than serving bad results.
        if store.integrity_check()? != "ok" {
            return Err(DomainError::storage("sqlite integrity_check failed"));
        }

        Ok(store)
    }

    fn encode(
        record: &ContextRecord,
    ) -> Result<(String, String, String, String, String, String, i64, String), DomainError> {
        let json = serde_json::to_string(record).map_err(storage_err)?;
        Ok((
            record.id.to_string(),
            record.project.clone(),
            record.kind.wire_name(),
            scope_str(record.scope).to_string(),
            timestamp(record.created_at),
            record.embedding.model.clone(),
            record.embedding.dims() as i64,
            json,
        ))
    }

    fn decode_row(row: &Row<'_>) -> rusqlite::Result<ContextRecord> {
        let json: String = row.get("record_json")?;
        serde_json::from_str(&json).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
        })
    }

    /// Bundled SQLite version number (for diagnostics/tests).
    pub fn sqlite_version_number(&self) -> i32 {
        rusqlite::version_number()
    }

    pub fn journal_mode(&self) -> Result<String, DomainError> {
        let conn = self.conn.lock();
        conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .map_err(storage_err)
    }

    /// `PRAGMA synchronous` value: 2 = FULL.
    pub fn synchronous(&self) -> Result<i64, DomainError> {
        let conn = self.conn.lock();
        conn.query_row("PRAGMA synchronous", [], |row| row.get(0))
            .map_err(storage_err)
    }

    pub fn schema_version(&self) -> Result<i32, DomainError> {
        let conn = self.conn.lock();
        conn.query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(storage_err)
    }

    pub fn busy_timeout_ms(&self) -> Result<i64, DomainError> {
        let conn = self.conn.lock();
        conn.query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .map_err(storage_err)
    }

    pub fn integrity_check(&self) -> Result<String, DomainError> {
        let conn = self.conn.lock();
        conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(storage_err)
    }
}

impl VectorStore for SqliteVectorStore {
    fn persist(&self, record: &ContextRecord) -> Result<(), DomainError> {
        let values = Self::encode(record)?;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO contexts (id, project, kind, scope, created_at, model, dimensions, record_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
                 project = excluded.project,
                 kind = excluded.kind,
                 scope = excluded.scope,
                 created_at = excluded.created_at,
                 model = excluded.model,
                 dimensions = excluded.dimensions,
                 record_json = excluded.record_json",
            params![
                values.0, values.1, values.2, values.3, values.4, values.5, values.6, values.7
            ],
        )
        .map_err(storage_err)?;
        Ok(())
    }

    fn insert_if_absent(&self, record: &ContextRecord) -> Result<bool, DomainError> {
        let values = Self::encode(record)?;
        let conn = self.conn.lock();
        let inserted = conn
            .execute(
                "INSERT INTO contexts (id, project, kind, scope, created_at, model, dimensions, record_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO NOTHING",
                params![
                    values.0, values.1, values.2, values.3, values.4, values.5, values.6, values.7
                ],
            )
            .map_err(storage_err)?;
        Ok(inserted > 0)
    }

    fn count(&self) -> Result<u64, DomainError> {
        let conn = self.conn.lock();
        conn.query_row("SELECT COUNT(*) FROM contexts", [], |row| {
            row.get::<_, i64>(0)
        })
        .map(|value| value.max(0) as u64)
        .map_err(storage_err)
    }

    fn search(
        &self,
        embedding: &ContextEmbedding,
        limit: usize,
        filters: &QueryFilters,
    ) -> Result<Vec<(ContextRecord, f32)>, DomainError> {
        let records = {
            let conn = self.conn.lock();
            let mut stmt = conn
                .prepare("SELECT record_json FROM contexts")
                .map_err(storage_err)?;
            let rows = stmt
                .query_map([], |row| Self::decode_row(row))
                .map_err(storage_err)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(storage_err)?
        };

        let mut scored: Vec<(ContextRecord, f32)> = Vec::new();
        for record in records {
            if !record.matches_filters(filters) {
                continue;
            }
            if record.embedding.model != embedding.model {
                return Err(DomainError::embedding(format!(
                    "model mismatch: query `{}` vs stored `{}`",
                    embedding.model, record.embedding.model
                )));
            }
            let score = cosine_similarity(&embedding.vector, &record.embedding.vector)?;
            scored.push((record, score));
        }

        // Descending score; ties broken deterministically by UUID.
        scored.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.id.cmp(&b.0.id)));
        scored.truncate(limit);

        Ok(scored)
    }

    fn recent(
        &self,
        project: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ContextSummary>, DomainError> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT record_json FROM contexts
                 WHERE (?1 IS NULL OR project = ?1)
                 ORDER BY created_at DESC
                 LIMIT ?2",
            )
            .map_err(storage_err)?;
        let rows = stmt
            .query_map(params![project, limit as i64], |row| {
                Self::decode_row(row).map(|record| record.as_summary())
            })
            .map_err(storage_err)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(storage_err)
    }

    fn projects(&self) -> Result<Vec<String>, DomainError> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT DISTINCT project FROM contexts ORDER BY project")
            .map_err(storage_err)?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(storage_err)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(storage_err)
    }

    fn ping(&self) -> Result<(), DomainError> {
        let conn = self.conn.lock();
        conn.query_row("SELECT 1", [], |_| Ok(()))
            .map_err(storage_err)
    }

    fn get(&self, id: &Uuid) -> Result<Option<ContextRecord>, DomainError> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT record_json FROM contexts WHERE id = ?1")
            .map_err(storage_err)?;
        let mut rows = stmt
            .query_map(params![id.to_string()], |row| Self::decode_row(row))
            .map_err(storage_err)?;
        match rows.next() {
            Some(Ok(record)) => Ok(Some(record)),
            Some(Err(err)) => Err(storage_err(err)),
            None => Ok(None),
        }
    }

    fn all(&self) -> Result<Vec<ContextRecord>, DomainError> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT record_json FROM contexts")
            .map_err(storage_err)?;
        let rows = stmt
            .query_map([], |row| Self::decode_row(row))
            .map_err(storage_err)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(storage_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ContextKind;

    fn record(project: &str, scope: MemoryScope, body: &str) -> ContextRecord {
        ContextRecord {
            id: Uuid::new_v4(),
            project: project.to_string(),
            ide: "test".to_string(),
            file_path: None,
            language: Some("rust".to_string()),
            summary: "summary".to_string(),
            body: body.to_string(),
            tags: vec!["fixture".to_string()],
            kind: ContextKind::Discussion,
            embedding: ContextEmbedding::new("ingat/simple-sha256-v1", vec![1.0, 0.0]),
            created_at: chrono::Utc::now(),
            scope,
            author: Some("tester".to_string()),
            provenance: Some("test".to_string()),
        }
    }

    #[test]
    fn pragmas_and_schema_are_asserted() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqliteVectorStore::open(dir.path().join("memory.sqlite")).unwrap();

        assert!(store.sqlite_version_number() >= SQLITE_REQUIRED_VERSION);
        assert_eq!(store.journal_mode().unwrap().to_lowercase(), "wal");
        assert_eq!(store.synchronous().unwrap(), 2, "synchronous must be FULL");
        assert_eq!(store.schema_version().unwrap(), SQLITE_SCHEMA_VERSION);
        assert_eq!(
            store.busy_timeout_ms().unwrap(),
            SQLITE_BUSY_TIMEOUT_MS as i64
        );
        assert_eq!(store.integrity_check().unwrap(), "ok");
    }

    #[test]
    fn equal_body_keeps_scope_and_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqliteVectorStore::open(dir.path().join("memory.sqlite")).unwrap();

        let personal = record("kode", MemoryScope::Personal, "same body");
        let team = record("kode", MemoryScope::Team, "same body");
        assert_ne!(personal.id, team.id);

        assert!(store.insert_if_absent(&personal).unwrap());
        assert!(store.insert_if_absent(&team).unwrap());
        // Re-inserting the same UUID must not mutate or double-count.
        assert!(!store.insert_if_absent(&personal).unwrap());
        assert_eq!(store.count().unwrap(), 2);

        let back = store.get(&personal.id).unwrap().unwrap();
        assert_eq!(back.scope, MemoryScope::Personal);
        assert_eq!(back.author.as_deref(), Some("tester"));
        assert_eq!(back.provenance.as_deref(), Some("test"));
        assert_eq!(back.body, "same body");
    }

    #[test]
    fn busy_owner_times_out_with_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("memory.sqlite");
        let holder = SqliteVectorStore::open(&path).unwrap();
        let writer = SqliteVectorStore::open(&path).unwrap();

        let conn = holder.conn.lock();
        conn.execute_batch("BEGIN IMMEDIATE").unwrap();

        // A second writer must time out with an error, never a fake success.
        let blocked = record("kode", MemoryScope::Personal, "blocked body");
        let result = writer.insert_if_absent(&blocked);
        assert!(result.is_err(), "blocked writer must return an error");

        conn.execute_batch("ROLLBACK").unwrap();
    }

    #[test]
    fn rejects_newer_schema_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("memory.sqlite");
        {
            let conn = Connection::open(&path).unwrap();
            conn.pragma_update(None, "user_version", SQLITE_SCHEMA_VERSION + 1)
                .unwrap();
        }
        let err = match SqliteVectorStore::open(&path) {
            Ok(_) => panic!("opening a newer schema must fail"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("unsupported schema version"));
    }
}
