//! Application layer wiring DTOs and services for Ingat.

pub mod dtos;
pub mod services;

#[cfg(feature = "sqlite-store")]
pub mod embedded;

pub use dtos::{
    EmbeddingBackendListResponse, EmbeddingBackendOption, HealthStatusResponse, ImportResponse,
    IngestContextRequest, LegacyExportLine, SearchRequest, SearchResponse, SummaryListResponse,
    UpdateEmbeddingBackendRequest, WireMemoryEntry,
};
#[cfg(feature = "sqlite-store")]
pub use embedded::{open_embedded, EmbeddedOptions, SIMPLE_SHA256_DIMENSIONS, SIMPLE_SHA256_MODEL};
pub use services::ContextService;
