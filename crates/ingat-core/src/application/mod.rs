//! Application layer wiring DTOs and services for Ingat.

pub mod dtos;
pub mod services;

pub use dtos::{
    EmbeddingBackendListResponse, EmbeddingBackendOption, HealthStatusResponse, ImportResponse,
    IngestContextRequest, SearchRequest, SearchResponse, SummaryListResponse,
    UpdateEmbeddingBackendRequest, WireMemoryEntry,
};
pub use services::ContextService;
