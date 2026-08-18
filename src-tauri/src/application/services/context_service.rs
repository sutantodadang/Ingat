use std::sync::Arc;

use chrono::Utc;
use sha2::{Digest, Sha256};
use ulid::Ulid;
use uuid::Uuid;

use crate::{
    application::dtos::{
        HealthStatusResponse, ImportResponse, IngestContextRequest, SearchRequest, SearchResponse,
        SearchResultDto, SummaryListResponse, WireMemoryEntry,
    },
    application::services::ContextApi,
    domain::{
        ContextEmbedding, ContextKind, ContextRecord, ContextSummary, DomainError, MemoryScope,
        QueryFilters, RetrievalQuery,
    },
};

const MAX_BODY_CHARS: usize = 16_000;
const MAX_SUMMARY_CHARS: usize = 640;

/// High level configuration shared by the service and its adapters.
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub embedding_model: String,
    pub default_limit: usize,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            embedding_model: "ingat/simple-hash".into(),
            default_limit: 8,
        }
    }
}

impl ServiceConfig {
    pub fn new(embedding_model: impl Into<String>, default_limit: usize) -> Self {
        Self {
            embedding_model: embedding_model.into(),
            default_limit: default_limit.max(1),
        }
    }

    pub fn with_model(embedding_model: impl Into<String>) -> Self {
        Self::new(embedding_model, Self::default().default_limit)
    }

    pub fn embedding_model(&self) -> &str {
        &self.embedding_model
    }
}

/// Abstraction over any embedding engine (FastEmbed, local HF, remote MCP bridge, etc).
pub trait EmbeddingEngine: Send + Sync {
    fn embed(&self, model: &str, text: &str) -> Result<Vec<f32>, DomainError>;

    fn dims(&self, _model: &str) -> Option<usize> {
        None
    }
}

/// Contract for the embedded vector storage engine.
pub trait VectorStore: Send + Sync {
    fn persist(&self, record: &ContextRecord) -> Result<(), DomainError>;

    fn search(
        &self,
        embedding: &ContextEmbedding,
        limit: usize,
        filters: &QueryFilters,
    ) -> Result<Vec<(ContextRecord, f32)>, DomainError>;

    fn recent(
        &self,
        project: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ContextSummary>, DomainError>;

    fn projects(&self) -> Result<Vec<String>, DomainError>;

    fn ping(&self) -> Result<(), DomainError>;

    fn get(&self, id: &Uuid) -> Result<Option<ContextRecord>, DomainError> {
        let _ = id;
        Err(DomainError::storage("get is not supported by this store"))
    }

    fn all(&self) -> Result<Vec<ContextRecord>, DomainError> {
        Err(DomainError::storage("all is not supported by this store"))
    }
}

/// The orchestrator responsible for validation, embedding, and delegating to storage.
pub struct ContextService {
    embedder: Arc<dyn EmbeddingEngine>,
    store: Arc<dyn VectorStore>,
    config: ServiceConfig,
}

impl ContextService {
    pub fn new(
        embedder: Arc<dyn EmbeddingEngine>,
        store: Arc<dyn VectorStore>,
        config: ServiceConfig,
    ) -> Self {
        Self {
            embedder,
            store,
            config,
        }
    }

    pub fn ingest(&self, payload: IngestContextRequest) -> Result<ContextSummary, DomainError> {
        self.validate_payload(&payload)?;

        let scope = payload.scope;
        let text_to_embed = format!("{}\n{}", payload.summary.trim(), payload.body.trim());
        let vector = self
            .embedder
            .embed(&self.config.embedding_model, &text_to_embed)?;
        let embedding = ContextEmbedding::new(&self.config.embedding_model, vector);

        let mut record = ContextRecord::new(
            payload.project,
            payload.ide,
            payload.file_path,
            payload.language,
            payload.summary,
            payload.body,
            payload.tags,
            payload.kind,
            embedding,
        );
        record.scope = scope;

        self.store.persist(&record)?;

        Ok(record.as_summary())
    }

    pub fn search(&self, request: SearchRequest) -> Result<SearchResponse, DomainError> {
        if request.prompt.trim().is_empty() {
            return Err(DomainError::validation("prompt cannot be empty"));
        }

        let RetrievalQuery {
            prompt,
            filters,
            limit,
        } = RetrievalQuery::from(request);

        let effective_limit = limit.clamp(1, 32);

        let query_vector = self
            .embedder
            .embed(&self.config.embedding_model, prompt.trim())?;
        let query_embedding = ContextEmbedding::new(&self.config.embedding_model, query_vector);

        let matches = self
            .store
            .search(&query_embedding, effective_limit, &filters)?;

        let results = matches
            .into_iter()
            .map(|(record, score)| SearchResultDto {
                id: record.id,
                project: record.project,
                summary: record.summary,
                body: record.body,
                tags: record.tags,
                kind: record.kind,
                score,
                created_at: record.created_at,
            })
            .collect();

        Ok(SearchResponse {
            query: prompt,
            results,
        })
    }

    pub fn history(
        &self,
        project: Option<String>,
        limit: Option<usize>,
    ) -> Result<SummaryListResponse, DomainError> {
        let capped_limit = limit.unwrap_or(self.config.default_limit).clamp(1, 50);
        let summaries = self.store.recent(project.as_deref(), capped_limit)?;

        Ok(SummaryListResponse { items: summaries })
    }

    pub fn projects(&self) -> Result<Vec<String>, DomainError> {
        self.store.projects()
    }

    pub fn embedding_dimensions(&self) -> Option<usize> {
        self.embedder.dims(self.config.embedding_model())
    }

    pub fn health(&self) -> Result<HealthStatusResponse, DomainError> {
        self.store.ping()?;

        let status = HealthStatusResponse {
            ok: true,
            message: "ready".into(),
            details: Some(format!(
                "model: {}, checked_at: {}",
                self.config.embedding_model,
                Utc::now()
            )),
        };

        Ok(status)
    }

    pub fn import_memories(
        &self,
        entries: Vec<WireMemoryEntry>,
    ) -> Result<ImportResponse, DomainError> {
        let mut existing_hashes: std::collections::HashSet<String> = self
            .store
            .all()?
            .iter()
            .map(|record| content_hash(&record.body))
            .collect();

        let mut imported = 0;
        let mut skipped = 0;

        for entry in entries {
            let content = entry.content.trim().to_string();
            if entry.id.trim().is_empty() || content.is_empty() {
                skipped += 1;
                continue;
            }

            let id = uuid_from_wire_id(entry.id.trim());
            if self.store.get(&id)?.is_some() {
                skipped += 1;
                continue;
            }

            let hash = content_hash(&content);
            if existing_hashes.contains(&hash) {
                skipped += 1;
                continue;
            }

            let summary: String = content
                .lines()
                .next()
                .unwrap_or_default()
                .chars()
                .take(MAX_SUMMARY_CHARS)
                .collect();
            let summary = if summary.trim().is_empty() {
                "(imported)".to_string()
            } else {
                summary
            };

            let vector = self
                .embedder
                .embed(&self.config.embedding_model, &content)?;
            let embedding = ContextEmbedding::new(&self.config.embedding_model, vector);

            let record = ContextRecord {
                id,
                project: crate::domain::models::sanitize_project(entry.repository),
                ide: "team-sync".to_string(),
                file_path: None,
                language: None,
                summary,
                body: content,
                tags: crate::domain::models::normalize_tags(entry.tags),
                kind: ContextKind::from_wire_name(entry.kind.trim()),
                embedding,
                created_at: entry.created_at,
                scope: MemoryScope::Team,
                author: entry.author,
                provenance: entry.provenance,
            };

            self.store.persist(&record)?;
            existing_hashes.insert(hash);
            imported += 1;
        }

        Ok(ImportResponse { imported, skipped })
    }

    pub fn export_memories(
        &self,
        scope: Option<MemoryScope>,
        repository: Option<String>,
    ) -> Result<Vec<WireMemoryEntry>, DomainError> {
        let repository = repository.map(crate::domain::models::sanitize_project);

        let mut entries: Vec<WireMemoryEntry> = self
            .store
            .all()?
            .into_iter()
            .filter(|record| scope.map_or(true, |s| record.scope == s))
            .filter(|record| {
                repository
                    .as_deref()
                    .map_or(true, |repo| record.project == repo)
            })
            .map(|record| WireMemoryEntry {
                v: 1,
                id: wire_id_for(&record.id),
                hash: Some(content_hash(&record.body)),
                kind: record.kind.wire_name(),
                content: record.body,
                tags: record.tags,
                author: Some(record.author.unwrap_or(record.ide)),
                repository: record.project,
                created_at: record.created_at,
                provenance: Some(record.provenance.unwrap_or_else(|| "user".to_string())),
            })
            .collect();

        entries.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        Ok(entries)
    }

    fn validate_payload(&self, payload: &IngestContextRequest) -> Result<(), DomainError> {
        if payload.project.trim().is_empty() {
            return Err(DomainError::validation("project is required"));
        }
        if payload.ide.trim().is_empty() {
            return Err(DomainError::validation("ide is required"));
        }
        if payload.summary.trim().is_empty() {
            return Err(DomainError::validation("summary is required"));
        }
        if payload.summary.chars().count() > MAX_SUMMARY_CHARS {
            return Err(DomainError::limit(format!(
                "summary cannot exceed {} characters",
                MAX_SUMMARY_CHARS
            )));
        }
        if payload.body.trim().is_empty() {
            return Err(DomainError::validation("body is required"));
        }
        if payload.body.chars().count() > MAX_BODY_CHARS {
            return Err(DomainError::limit(format!(
                "body cannot exceed {} characters",
                MAX_BODY_CHARS
            )));
        }
        if payload.tags.len() > crate::domain::models::MAX_TAGS {
            return Err(DomainError::limit(format!(
                "tags cannot exceed {} entries",
                crate::domain::models::MAX_TAGS
            )));
        }
        Self::validate_kind(&payload.kind)
    }

    fn validate_kind(kind: &ContextKind) -> Result<(), DomainError> {
        match kind {
            ContextKind::Other(label) if label.trim().is_empty() => {
                Err(DomainError::validation("custom kind label cannot be empty"))
            }
            _ => Ok(()),
        }
    }
}

fn content_hash(body: &str) -> String {
    let digest = Sha256::digest(body.as_bytes());
    format!("sha256:{:x}", digest)
}

fn uuid_from_wire_id(id: &str) -> Uuid {
    match Ulid::from_string(id) {
        Ok(ulid) => Uuid::from_u128(u128::from(ulid)),
        Err(_) => {
            // ponytail: non-ULID ids map via sha256 so import stays deterministic
            let digest = Sha256::digest(id.as_bytes());
            let mut bytes = [0u8; 16];
            bytes.copy_from_slice(&digest[..16]);
            Uuid::from_bytes(bytes)
        }
    }
}

fn wire_id_for(record_id: &Uuid) -> String {
    Ulid::from(record_id.as_u128()).to_string()
}

impl ContextApi for ContextService {
    fn ingest(&self, payload: IngestContextRequest) -> Result<ContextSummary, DomainError> {
        ContextService::ingest(self, payload)
    }

    fn search(&self, request: SearchRequest) -> Result<SearchResponse, DomainError> {
        ContextService::search(self, request)
    }

    fn history(
        &self,
        project: Option<String>,
        limit: Option<usize>,
    ) -> Result<SummaryListResponse, DomainError> {
        ContextService::history(self, project, limit)
    }

    fn projects(&self) -> Result<Vec<String>, DomainError> {
        ContextService::projects(self)
    }

    fn health(&self) -> Result<HealthStatusResponse, DomainError> {
        ContextService::health(self)
    }

    fn embedding_dimensions(&self) -> Option<usize> {
        ContextService::embedding_dimensions(self)
    }

    fn import_memories(&self, entries: Vec<WireMemoryEntry>) -> Result<ImportResponse, DomainError> {
        ContextService::import_memories(self, entries)
    }

    fn export_memories(
        &self,
        scope: Option<MemoryScope>,
        repository: Option<String>,
    ) -> Result<Vec<WireMemoryEntry>, DomainError> {
        ContextService::export_memories(self, scope, repository)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct FakeEmbeddingEngine;

    impl EmbeddingEngine for FakeEmbeddingEngine {
        fn embed(&self, _model: &str, _text: &str) -> Result<Vec<f32>, DomainError> {
            Ok(vec![1.0, 0.0])
        }
    }

    #[derive(Default)]
    struct InMemoryStore {
        records: Mutex<HashMap<Uuid, ContextRecord>>,
    }

    impl VectorStore for InMemoryStore {
        fn persist(&self, record: &ContextRecord) -> Result<(), DomainError> {
            self.records
                .lock()
                .unwrap()
                .insert(record.id, record.clone());
            Ok(())
        }

        fn search(
            &self,
            _embedding: &ContextEmbedding,
            _limit: usize,
            _filters: &QueryFilters,
        ) -> Result<Vec<(ContextRecord, f32)>, DomainError> {
            Ok(Vec::new())
        }

        fn recent(
            &self,
            _project: Option<&str>,
            _limit: usize,
        ) -> Result<Vec<ContextSummary>, DomainError> {
            Ok(Vec::new())
        }

        fn projects(&self) -> Result<Vec<String>, DomainError> {
            Ok(Vec::new())
        }

        fn ping(&self) -> Result<(), DomainError> {
            Ok(())
        }

        fn get(&self, id: &Uuid) -> Result<Option<ContextRecord>, DomainError> {
            Ok(self.records.lock().unwrap().get(id).cloned())
        }

        fn all(&self) -> Result<Vec<ContextRecord>, DomainError> {
            Ok(self.records.lock().unwrap().values().cloned().collect())
        }
    }

    fn make_service() -> ContextService {
        ContextService::new(
            Arc::new(FakeEmbeddingEngine),
            Arc::new(InMemoryStore::default()),
            ServiceConfig::default(),
        )
    }

    fn wire_entry(id: &str, content: &str) -> WireMemoryEntry {
        WireMemoryEntry {
            v: 1,
            id: id.to_string(),
            hash: None,
            kind: "discussion".to_string(),
            content: content.to_string(),
            tags: vec!["auth".to_string()],
            author: Some("alice".to_string()),
            repository: "kode".to_string(),
            created_at: Utc::now(),
            provenance: Some("user".to_string()),
        }
    }

    const ULID_A: &str = "01J8ZZZZZZZZZZZZZZZZZZZZZZ";
    const ULID_B: &str = "01J8ZZZZZZZZZZZZZZZZZZZZZY";

    #[test]
    fn import_two_entries_succeeds() {
        let service = make_service();
        let entries = vec![
            wire_entry(ULID_A, "first memory content"),
            wire_entry(ULID_B, "second memory content"),
        ];

        let response = service.import_memories(entries).unwrap();
        assert_eq!(response.imported, 2);
        assert_eq!(response.skipped, 0);
    }

    #[test]
    fn reimporting_same_entries_is_idempotent() {
        let service = make_service();
        let entries = vec![
            wire_entry(ULID_A, "first memory content"),
            wire_entry(ULID_B, "second memory content"),
        ];

        service.import_memories(entries.clone()).unwrap();
        let response = service.import_memories(entries).unwrap();

        assert_eq!(response.imported, 0);
        assert_eq!(response.skipped, 2);
    }

    #[test]
    fn export_round_trips_wire_ids() {
        let service = make_service();
        let entries = vec![
            wire_entry(ULID_A, "first memory content"),
            wire_entry(ULID_B, "second memory content"),
        ];
        service.import_memories(entries).unwrap();

        let exported = service
            .export_memories(Some(MemoryScope::Team), None)
            .unwrap();

        assert_eq!(exported.len(), 2);
        let exported_ids: Vec<&str> = exported.iter().map(|e| e.id.as_str()).collect();
        assert!(exported_ids.contains(&ULID_A));
        assert!(exported_ids.contains(&ULID_B));
    }

    #[test]
    fn ingest_with_team_scope_appears_in_team_export() {
        let service = make_service();
        let payload = IngestContextRequest {
            project: "kode".to_string(),
            ide: "vscode".to_string(),
            file_path: None,
            language: None,
            summary: "summary line".to_string(),
            body: "body content".to_string(),
            tags: vec![],
            kind: ContextKind::Discussion,
            scope: MemoryScope::Team,
        };

        service.ingest(payload).unwrap();

        let exported = service
            .export_memories(Some(MemoryScope::Team), None)
            .unwrap();

        assert_eq!(exported.len(), 1);
        assert_eq!(exported[0].content, "body content");
    }
}
