//! No-op embedding engine for remote mode.
//!
//! When running in remote mode, all embedding operations are proxied to the
//! remote mcp-service. This engine exists only to satisfy the type system
//! and should never actually be called for embedding operations.

use crate::{application::services::EmbeddingEngine, domain::DomainError};

/// A no-op embedding engine that doesn't perform any actual embedding.
///
/// This is used in remote mode where all embedding operations are handled
/// by the remote mcp-service. If `embed` is called, it will return an error
/// indicating that the operation should have been proxied to the remote service.
pub struct NoOpEmbeddingEngine {
    model_name: String,
    dimensions: usize,
}

impl NoOpEmbeddingEngine {
    /// Create a new no-op embedding engine with the specified model name.
    pub fn new(model_name: impl Into<String>, dimensions: usize) -> Self {
        Self {
            model_name: model_name.into(),
            dimensions,
        }
    }

    /// Create a no-op engine with default parameters for remote mode.
    pub fn for_remote_mode() -> Self {
        Self::new("remote-proxy", 384)
    }
}

impl Default for NoOpEmbeddingEngine {
    fn default() -> Self {
        Self::for_remote_mode()
    }
}

impl EmbeddingEngine for NoOpEmbeddingEngine {
    fn embed(&self, _model: &str, _text: &str) -> Result<Vec<f32>, DomainError> {
        // In remote mode, embedding should never be called locally
        // The RemoteVectorStore handles all operations including embedding
        Err(DomainError::embedding(
            "embedding operation called on no-op engine; this should be handled by remote service",
        ))
    }

    fn dims(&self, _model: &str) -> Option<usize> {
        Some(self.dimensions)
    }
}
