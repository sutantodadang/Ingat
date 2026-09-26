use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use bincode::Options;
use parking_lot::Mutex;
use serde::de::DeserializeOwned;
use sled::{Config, Db, IVec, Tree};
use uuid::Uuid;

use crate::{
    application::services::VectorStore,
    domain::{ContextEmbedding, ContextRecord, ContextSummary, DomainError, QueryFilters},
};

use super::similarity::cosine_similarity;

const CONTEXTS_TREE: &str = "contexts";

/// Pre-scope on-disk layout (records written before v0.1.6). Bincode is not
/// self-describing, so appended fields require this explicit fallback.
#[derive(serde::Deserialize)]
struct LegacyContextRecord {
    id: Uuid,
    project: String,
    ide: String,
    file_path: Option<String>,
    language: Option<String>,
    summary: String,
    body: String,
    tags: Vec<String>,
    kind: crate::domain::ContextKind,
    embedding: ContextEmbedding,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<LegacyContextRecord> for ContextRecord {
    fn from(legacy: LegacyContextRecord) -> Self {
        ContextRecord {
            id: legacy.id,
            project: legacy.project,
            ide: legacy.ide,
            file_path: legacy.file_path,
            language: legacy.language,
            summary: legacy.summary,
            body: legacy.body,
            tags: legacy.tags,
            kind: legacy.kind,
            embedding: legacy.embedding,
            created_at: legacy.created_at,
            scope: crate::domain::MemoryScope::default(),
            author: None,
            provenance: None,
        }
    }
}

/// Embedded vector store backed by `sled`.
///
/// This adapter keeps the implementation intentionally simple by storing full
/// `ContextRecord` payloads in a single tree. Vector similarity is performed
/// in-memory using cosine similarity, which is acceptable for moderate data
/// volumes and keeps the design embeddable without additional services.
///
/// For larger datasets the same trait can be satisfied by a more sophisticated
/// index without touching callers.
pub struct SledVectorStore {
    db: Db,
    contexts: Tree,
    _data_dir: PathBuf,
    write_lock: Mutex<()>,
}

impl SledVectorStore {
    /// Opens (or creates) a sled database rooted at `data_dir`.
    pub fn open(data_dir: impl AsRef<Path>) -> Result<Self, DomainError> {
        let dir = data_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir).map_err(|err| {
            DomainError::storage(format!("failed to create data directory {:?}: {err}", dir))
        })?;

        let db = Config::default()
            .path(&dir)
            .cache_capacity(64 * 1024 * 1024)
            .mode(sled::Mode::HighThroughput)
            .open()
            .map_err(|err| DomainError::storage(format!("failed to open sled db: {err}")))?;

        let contexts = db
            .open_tree(CONTEXTS_TREE)
            .map_err(|err| DomainError::storage(format!("failed to open contexts tree: {err}")))?;

        Ok(Self {
            db,
            contexts,
            _data_dir: dir,
            write_lock: Mutex::new(()),
        })
    }

    fn serialize<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, DomainError> {
        bincode::options()
            .with_fixint_encoding()
            .allow_trailing_bytes()
            .serialize(value)
            .map_err(|err| DomainError::storage(format!("serialization error: {err}")))
    }

    fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, DomainError> {
        bincode::options()
            .with_fixint_encoding()
            .allow_trailing_bytes()
            .deserialize(bytes)
            .map_err(|err| DomainError::storage(format!("deserialization error: {err}")))
    }

    fn encode_key(id: &Uuid) -> [u8; 16] {
        *id.as_bytes()
    }

    fn decode_record(bytes: &IVec) -> Result<ContextRecord, DomainError> {
        // Try the current layout first: allow_trailing_bytes would let the
        // legacy layout silently swallow the new fields if tried first.
        match Self::deserialize::<ContextRecord>(bytes.as_ref()) {
            Ok(record) => Ok(record),
            Err(_) => {
                Self::deserialize::<LegacyContextRecord>(bytes.as_ref()).map(ContextRecord::from)
            }
        }
    }

    fn record_matches_filters(record: &ContextRecord, filters: &QueryFilters) -> bool {
        record.matches_filters(filters)
    }
}

impl VectorStore for SledVectorStore {
    fn persist(&self, record: &ContextRecord) -> Result<(), DomainError> {
        let _guard = self.write_lock.lock();

        let bytes = Self::serialize(record)?;
        self.contexts
            .insert(Self::encode_key(&record.id), bytes)
            .map_err(|err| DomainError::storage(format!("failed to persist context: {err}")))?;

        self.contexts
            .flush()
            .map_err(|err| DomainError::storage(format!("failed to flush contexts: {err}")))?;

        Ok(())
    }

    fn insert_if_absent(&self, record: &ContextRecord) -> Result<bool, DomainError> {
        let _guard = self.write_lock.lock();

        let key = Self::encode_key(&record.id);
        if self
            .contexts
            .contains_key(key)
            .map_err(|err| DomainError::storage(format!("failed to read context record: {err}")))?
        {
            return Ok(false);
        }

        let bytes = Self::serialize(record)?;
        self.contexts
            .insert(key, bytes)
            .map_err(|err| DomainError::storage(format!("failed to persist context: {err}")))?;
        self.contexts
            .flush()
            .map_err(|err| DomainError::storage(format!("failed to flush contexts: {err}")))?;

        Ok(true)
    }

    fn count(&self) -> Result<u64, DomainError> {
        Ok(self.contexts.len() as u64)
    }

    fn search(
        &self,
        embedding: &ContextEmbedding,
        limit: usize,
        filters: &QueryFilters,
    ) -> Result<Vec<(ContextRecord, f32)>, DomainError> {
        let mut scored: Vec<(ContextRecord, f32)> = Vec::new();

        for entry in self.contexts.iter() {
            let (_, value) = entry.map_err(|err| {
                DomainError::storage(format!("failed to read context record: {err}"))
            })?;
            let record = Self::decode_record(&value)?;

            if !Self::record_matches_filters(&record, filters) {
                continue;
            }

            let score = cosine_similarity(&embedding.vector, &record.embedding.vector)?;

            scored.push((record, score));
        }

        scored.sort_by(|a, b| b.1.total_cmp(&a.1));
        scored.truncate(limit);

        Ok(scored)
    }

    fn recent(
        &self,
        project: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ContextSummary>, DomainError> {
        let mut items: Vec<ContextSummary> = Vec::new();

        for entry in self.contexts.iter() {
            let (_, value) = entry.map_err(|err| {
                DomainError::storage(format!("failed to read context record: {err}"))
            })?;
            let record = Self::decode_record(&value)?;

            if let Some(project_ref) = project {
                if record.project != project_ref {
                    continue;
                }
            }

            items.push(record.as_summary());
        }

        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        items.truncate(limit);

        Ok(items)
    }

    fn projects(&self) -> Result<Vec<String>, DomainError> {
        let mut unique = BTreeSet::new();

        for entry in self.contexts.iter() {
            let (_, value) = entry.map_err(|err| {
                DomainError::storage(format!("failed to read context record: {err}"))
            })?;
            let record = Self::decode_record(&value)?;
            unique.insert(record.project);
        }

        Ok(unique.into_iter().collect())
    }

    fn ping(&self) -> Result<(), DomainError> {
        self.db
            .flush()
            .map_err(|err| DomainError::storage(format!("failed to flush db: {err}")))?;

        Ok(())
    }

    fn get(&self, id: &Uuid) -> Result<Option<ContextRecord>, DomainError> {
        let value = self
            .contexts
            .get(Self::encode_key(id))
            .map_err(|err| DomainError::storage(format!("failed to read context record: {err}")))?;

        value.map(|bytes| Self::decode_record(&bytes)).transpose()
    }

    fn all(&self) -> Result<Vec<ContextRecord>, DomainError> {
        let mut records = Vec::new();

        for entry in self.contexts.iter() {
            let (_, value) = entry.map_err(|err| {
                DomainError::storage(format!("failed to read context record: {err}"))
            })?;
            records.push(Self::decode_record(&value)?);
        }

        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ContextEmbedding, ContextKind, MemoryScope};

    /// Mirrors the pre-scope on-disk layout so the fixture encodes exactly the
    /// bytes an older Ingat release would have written.
    #[derive(serde::Serialize)]
    struct LegacyFixture {
        id: Uuid,
        project: String,
        ide: String,
        file_path: Option<String>,
        language: Option<String>,
        summary: String,
        body: String,
        tags: Vec<String>,
        kind: ContextKind,
        embedding: ContextEmbedding,
        created_at: chrono::DateTime<chrono::Utc>,
    }

    #[test]
    fn legacy_records_decode_as_personal() {
        let dir = tempfile::tempdir().expect("temporary fixture directory");
        let store = SledVectorStore::open(dir.path()).expect("open temporary sled store");

        let id = Uuid::new_v4();
        let legacy = LegacyFixture {
            id,
            project: "kode".to_string(),
            ide: "vscode".to_string(),
            file_path: Some("src/main.rs".to_string()),
            language: Some("rust".to_string()),
            summary: "legacy summary".to_string(),
            body: "legacy body".to_string(),
            tags: vec!["legacy".to_string()],
            kind: ContextKind::Discussion,
            embedding: ContextEmbedding::new("ingat/simple-hash", vec![1.0, 0.0]),
            created_at: chrono::Utc::now(),
        };

        let bytes = bincode::options()
            .with_fixint_encoding()
            .allow_trailing_bytes()
            .serialize(&legacy)
            .expect("encode legacy fixture");
        store
            .contexts
            .insert(SledVectorStore::encode_key(&id), bytes)
            .expect("write legacy fixture");

        let decoded = store.get(&id).expect("read").expect("record present");
        assert_eq!(decoded.scope, MemoryScope::Personal);
        assert!(decoded.author.is_none());
        assert!(decoded.provenance.is_none());
        assert_eq!(decoded.project, "kode");
        assert_eq!(decoded.body, "legacy body");
    }
}

#[cfg(all(test, feature = "legacy-export"))]
mod export_tests {
    use std::collections::HashMap;

    use super::*;
    use crate::application::dtos::LegacyExportLine;
    use crate::domain::{ContextKind, MemoryScope};

    #[derive(serde::Serialize)]
    struct LegacyFixture {
        id: Uuid,
        project: String,
        ide: String,
        file_path: Option<String>,
        language: Option<String>,
        summary: String,
        body: String,
        tags: Vec<String>,
        kind: ContextKind,
        embedding: ContextEmbedding,
        created_at: chrono::DateTime<chrono::Utc>,
    }

    fn legacy_bytes(id: Uuid, body: &str) -> Vec<u8> {
        let record = LegacyFixture {
            id,
            project: "legacy-project".to_string(),
            ide: "old-ide".to_string(),
            file_path: Some("old/path.rs".to_string()),
            language: Some("rust".to_string()),
            summary: "legacy summary".to_string(),
            body: body.to_string(),
            tags: vec!["legacy".to_string()],
            kind: ContextKind::Discussion,
            embedding: ContextEmbedding::new("ingat/simple-hash", vec![1.0, 0.0]),
            created_at: chrono::Utc::now(),
        };
        bincode::options()
            .with_fixint_encoding()
            .allow_trailing_bytes()
            .serialize(&record)
            .unwrap()
    }

    #[test]
    fn export_preserves_non_vector_fields_and_source() {
        let dir = tempfile::tempdir().unwrap();
        let store_path = dir.path().join("store");
        let current_id = Uuid::new_v4();
        let legacy_id = Uuid::new_v4();

        {
            let store = SledVectorStore::open(&store_path).unwrap();
            let current = ContextRecord {
                id: current_id,
                project: "kode".to_string(),
                ide: "vscode".to_string(),
                file_path: Some("src/lib.rs".to_string()),
                language: Some("rust".to_string()),
                summary: "current summary".to_string(),
                body: "current body".to_string(),
                tags: vec!["a".to_string(), "b".to_string()],
                kind: ContextKind::CodeSnippet,
                embedding: ContextEmbedding::new("ingat/simple-hash", vec![1.0, 0.0]),
                created_at: chrono::Utc::now(),
                scope: MemoryScope::Team,
                author: Some("alice".to_string()),
                provenance: Some("user".to_string()),
            };
            store.persist(&current).unwrap();
            store
                .contexts
                .insert(
                    SledVectorStore::encode_key(&legacy_id),
                    legacy_bytes(legacy_id, "legacy body"),
                )
                .unwrap();
        }

        let out = dir.path().join("export.jsonl");
        let count =
            crate::infrastructure::storage::export::export_legacy_jsonl(&store_path, &out).unwrap();
        assert_eq!(count, 2);

        let content = std::fs::read_to_string(&out).unwrap();
        let lines: Vec<LegacyExportLine> = content
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let by_id: HashMap<Uuid, ContextRecord> = lines
            .iter()
            .map(|line| (line.record.id, line.record.clone()))
            .collect();

        let current = by_id.get(&current_id).expect("current record exported");
        assert_eq!(current.project, "kode");
        assert_eq!(current.ide, "vscode");
        assert_eq!(current.file_path.as_deref(), Some("src/lib.rs"));
        assert_eq!(current.language.as_deref(), Some("rust"));
        assert_eq!(current.summary, "current summary");
        assert_eq!(current.body, "current body");
        assert_eq!(current.tags, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(current.kind, ContextKind::CodeSnippet);
        assert_eq!(current.scope, MemoryScope::Team);
        assert_eq!(current.author.as_deref(), Some("alice"));
        assert_eq!(current.provenance.as_deref(), Some("user"));

        let legacy = by_id.get(&legacy_id).expect("legacy record exported");
        assert_eq!(legacy.scope, MemoryScope::Personal);
        assert_eq!(legacy.body, "legacy body");

        // Source store is untouched.
        let reopened = SledVectorStore::open(&store_path).unwrap();
        assert_eq!(reopened.all().unwrap().len(), 2);
    }

    #[test]
    fn export_refuses_locked_store_and_keeps_output() {
        let dir = tempfile::tempdir().unwrap();
        let store_path = dir.path().join("store");
        let out = dir.path().join("export.jsonl");
        std::fs::write(&out, b"sentinel\n").unwrap();

        // Hold the sled ownership lock for the duration of the export attempt.
        let _guard = SledVectorStore::open(&store_path).unwrap();

        let err = crate::infrastructure::storage::export::export_legacy_jsonl(&store_path, &out)
            .unwrap_err();
        assert!(err.to_string().contains("sled"), "unexpected error: {err}");
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "sentinel\n");
        assert!(!dir.path().join("export.jsonl.tmp").exists());
    }
}
