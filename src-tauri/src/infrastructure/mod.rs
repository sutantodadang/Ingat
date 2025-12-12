//! Infrastructure layer wiring concrete adapters (embeddings, storage, etc).

pub mod embeddings;
pub mod http_client;
pub mod storage;

#[cfg(feature = "fastembed-engine")]
pub use embeddings::FastEmbedEngine;
pub use embeddings::NoOpEmbeddingEngine;
pub use embeddings::SimpleEmbedEngine;
pub use http_client::{check_service_availability, RemoteVectorStore};
pub use storage::SledVectorStore;
