//! Infrastructure layer wiring concrete adapters (embeddings, storage, etc).

pub mod embeddings;
pub mod storage;

#[cfg(feature = "fastembed-engine")]
pub use embeddings::FastEmbedEngine;
pub use embeddings::NoOpEmbeddingEngine;
pub use embeddings::Sha256EmbedEngine;
pub use embeddings::SimpleEmbedEngine;
#[cfg(feature = "legacy-export")]
pub use storage::export_legacy_jsonl;
#[cfg(feature = "sled-store")]
pub use storage::SledVectorStore;
#[cfg(feature = "sqlite-store")]
pub use storage::SqliteVectorStore;
