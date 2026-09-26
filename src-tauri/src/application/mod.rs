//! Application layer re-exported from `ingat-core`.
//!
//! The DTOs and services (including `ContextService`, `ContextApi`,
//! `VectorStore` and `EmbeddingEngine`) live in the headless core crate. This
//! shim keeps the desktop/server call-sites on their existing `crate::application`
//! paths.

pub use ingat_core::application::*;
