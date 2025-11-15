use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::dtos::{
        HealthStatusResponse, IngestContextRequest, SearchRequest, SearchResponse, SearchResultDto,
        SummaryListResponse,
    },
    domain::{
        ContextEmbedding, ContextKind, ContextRecord, ContextSummary, DomainError, QueryFilters,
        RetrievalQuery,
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

        let text_to_embed = format!("{}\n{}", payload.summary.trim(), payload.body.trim());
        let vector = self
            .embedder
            .embed(&self.config.embedding_model, &text_to_embed)?;
        let embedding = ContextEmbedding::new(&self.config.embedding_model, vector);

        let record = ContextRecord::new(
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
