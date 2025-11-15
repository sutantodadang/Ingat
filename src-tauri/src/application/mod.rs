//! Application layer wiring DTOs and services for Ingat.

pub mod dtos;
pub mod services;

pub use dtos::{
    EmbeddingBackendListResponse, EmbeddingBackendOption, HealthStatusResponse,
    IngestContextRequest, SearchRequest, SearchResponse, SummaryListResponse,
    UpdateEmbeddingBackendRequest,
};
pub use services::ContextService;
