//! Storage adapters for Ingat.
//!
//! This module currently exposes the embedded sled-backed vector store that
//! powers semantic retrieval and history listings. The `sqlite-store` feature
//! is reserved for the storage follow-up and adds no adapter yet.

#[cfg(feature = "sled-store")]
pub mod sled_store;

#[cfg(feature = "sled-store")]
pub use sled_store::SledVectorStore;
