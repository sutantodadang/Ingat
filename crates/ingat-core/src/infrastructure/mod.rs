//! Infrastructure layer wiring concrete adapters (embeddings, storage, etc).

pub mod embeddings;
pub mod storage;

#[cfg(feature = "fastembed-engine")]
pub use embeddings::FastEmbedEngine;
pub use embeddings::NoOpEmbeddingEngine;
pub use embeddings::SimpleEmbedEngine;
#[cfg(feature = "sled-store")]
pub use storage::SledVectorStore;
