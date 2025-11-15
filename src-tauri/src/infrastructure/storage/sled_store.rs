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

const CONTEXTS_TREE: &str = "contexts";

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
        Self::deserialize(bytes.as_ref())
    }

    fn cosine_similarity(query: &[f32], candidate: &[f32]) -> Result<f32, DomainError> {
        if query.len() != candidate.len() {
            return Err(DomainError::embedding(format!(
                "embedding dimension mismatch: query {} vs candidate {}",
                query.len(),
                candidate.len()
            )));
        }

        let mut dot = 0.0f32;
        let mut q_norm = 0.0f32;
        let mut c_norm = 0.0f32;

        for (q, c) in query.iter().zip(candidate.iter()) {
            dot += q * c;
            q_norm += q * q;
            c_norm += c * c;
        }

        let denom = q_norm.sqrt() * c_norm.sqrt();
        if denom == 0.0 {
            return Err(DomainError::embedding(
                "cannot compute cosine similarity with zero vector",
            ));
        }

        Ok((dot / denom).clamp(-1.0, 1.0))
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

            let score = Self::cosine_similarity(&embedding.vector, &record.embedding.vector)?;

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
}
