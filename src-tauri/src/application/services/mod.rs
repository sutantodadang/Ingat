//! Service layer orchestrating domain operations and infrastructure adapters.

mod context_service;

pub use context_service::{ContextService, EmbeddingEngine, ServiceConfig, VectorStore};
