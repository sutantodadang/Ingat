use crate::application::dtos::{
    HealthStatusResponse, IngestContextRequest, SearchRequest, SearchResponse, SummaryListResponse,
};
use crate::domain::{ContextSummary, DomainError};

/// Common interface used by both local (embedded) and remote (HTTP proxied) context backends.
///
/// This keeps call-sites (Tauri commands, MCP runtime) agnostic to whether Ingat is using
/// an embedded store or a long-running `mcp-service` process.
pub trait ContextApi: Send + Sync {
    fn ingest(&self, payload: IngestContextRequest) -> Result<ContextSummary, DomainError>;

    fn search(&self, request: SearchRequest) -> Result<SearchResponse, DomainError>;

    fn history(
        &self,
        project: Option<String>,
        limit: Option<usize>,
    ) -> Result<SummaryListResponse, DomainError>;

    fn projects(&self) -> Result<Vec<String>, DomainError>;

    fn health(&self) -> Result<HealthStatusResponse, DomainError>;

    fn embedding_dimensions(&self) -> Option<usize> {
        None
    }
}
