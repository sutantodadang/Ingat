//! Domain layer: core business entities and value objects for Ingat.

pub mod errors;
pub mod models;

pub use errors::DomainError;
pub use models::{
    ContextEmbedding, ContextKind, ContextRecord, ContextSummary, QueryFilters, RetrievalQuery,
};
