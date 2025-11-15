//! Storage adapters for Ingat.
//!
//! This module currently exposes the embedded sled-backed vector store
//! that powers semantic retrieval and history listings.

pub mod sled_store;

pub use sled_store::SledVectorStore;
