use chrono::{DateTime, Utc};
#[cfg(feature = "mcp-server")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{ContextKind, ContextSummary, QueryFilters, RetrievalQuery};

/// Payload accepted from MCP clients or the UI when persisting a new context item.
#[cfg_attr(feature = "mcp-server", derive(JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestContextRequest {
    pub project: String,
    pub ide: String,
    pub file_path: Option<String>,
    pub language: Option<String>,
    pub summary: String,
    pub body: String,
    pub tags: Vec<String>,
    #[serde(default)]
    pub kind: ContextKind,
}

/// DTO bridging the UI search form and the application layer.
#[cfg_attr(feature = "mcp-server", derive(JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub prompt: String,
    #[serde(default)]
    pub filters: QueryFilters,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

impl From<SearchRequest> for RetrievalQuery {
    fn from(value: SearchRequest) -> Self {
        Self {
            prompt: value.prompt,
            filters: value.filters,
            limit: value.limit,
        }
    }
}

/// Result row returned from semantic retrieval.
#[cfg_attr(feature = "mcp-server", derive(JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultDto {
    #[cfg_attr(feature = "mcp-server", schemars(with = "String"))]
    pub id: Uuid,
    pub project: String,
    pub summary: String,
    pub body: String,
    pub tags: Vec<String>,
    pub kind: ContextKind,
    pub score: f32,
    pub created_at: DateTime<Utc>,
}

/// Response envelope for search operations.
#[cfg_attr(feature = "mcp-server", derive(JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResultDto>,
}

/// Simple projection for timeline/history listings.
#[cfg_attr(feature = "mcp-server", derive(JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryListResponse {
    pub items: Vec<ContextSummary>,
}

/// Health/readiness report for diagnostics.
#[cfg_attr(feature = "mcp-server", derive(JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatusResponse {
    pub ok: bool,
    pub message: String,
    pub details: Option<String>,
}

#[cfg_attr(feature = "mcp-server", derive(JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingBackendOption {
    pub id: String,
    pub label: String,
    pub description: String,
    pub model: String,
    pub dimensions: Option<usize>,
    pub feature_gated: bool,
}

#[cfg_attr(feature = "mcp-server", derive(JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingBackendListResponse {
    pub active: String,
    pub options: Vec<EmbeddingBackendOption>,
}

#[cfg_attr(feature = "mcp-server", derive(JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEmbeddingBackendRequest {
    pub backend_id: String,
    pub model_override: Option<String>,
}

const fn default_limit() -> usize {
    8
}
