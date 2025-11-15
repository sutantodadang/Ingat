use thiserror::Error;

/// Domain-level errors shared across application components.
#[derive(Debug, Error)]
pub enum DomainError {
    /// The incoming payload missed a required field or violated invariants.
    #[error("validation error: {0}")]
    Validation(String),

    /// Input exceeded guard rails such as maximum length or count.
    #[error("limit exceeded: {0}")]
    LimitExceeded(String),

    /// Requested entity was not found locally.
    #[error("not found: {0}")]
    NotFound(String),

    /// Catch-all for storage-related failures we don't want to leak directly.
    #[error("storage failure: {0}")]
    Storage(String),

    /// Vector store incompatibility (e.g., dimension mismatch).
    #[error("embedding mismatch: {0}")]
    Embedding(String),

    /// Any other unexpected failure.
    #[error("unexpected error: {0}")]
    Other(String),
}

impl DomainError {
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    pub fn limit(msg: impl Into<String>) -> Self {
        Self::LimitExceeded(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn storage(msg: impl Into<String>) -> Self {
        Self::Storage(msg.into())
    }

    pub fn embedding(msg: impl Into<String>) -> Self {
        Self::Embedding(msg.into())
    }

    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}
