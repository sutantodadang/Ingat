//! Convenience constructor for the concurrent embedded (SQLite) store.
//!
//! Wires the SQLite adapter to the stable `ingat/simple-sha256-v1` hash model
//! with an explicit database path. No OS path discovery, service probing or
//! global runtime is involved.

use std::path::PathBuf;
use std::sync::Arc;

use crate::application::services::{ContextService, ServiceConfig};
use crate::domain::DomainError;
use crate::infrastructure::{Sha256EmbedEngine, SqliteVectorStore};

/// Stable, process-independent hash model used by [`open_embedded`].
pub const SIMPLE_SHA256_MODEL: &str = "ingat/simple-sha256-v1";
/// Vector dimensions for [`SIMPLE_SHA256_MODEL`].
pub const SIMPLE_SHA256_DIMENSIONS: usize = 256;
/// Default retrieval limit for the embedded store.
pub const EMBEDDED_DEFAULT_LIMIT: usize = 8;

/// Explicit options for [`open_embedded`].
#[derive(Debug, Clone)]
pub struct EmbeddedOptions {
    pub database_path: PathBuf,
}

/// Opens the concurrent SQLite store at `options.database_path` and returns a
/// ready [`ContextService`].
pub fn open_embedded(options: EmbeddedOptions) -> Result<ContextService, DomainError> {
    let store = Arc::new(SqliteVectorStore::open(&options.database_path)?);
    let embedder = Arc::new(Sha256EmbedEngine::try_new(
        SIMPLE_SHA256_MODEL,
        SIMPLE_SHA256_DIMENSIONS,
    )?);
    Ok(ContextService::new(
        embedder,
        store,
        ServiceConfig::new(SIMPLE_SHA256_MODEL, EMBEDDED_DEFAULT_LIMIT),
    ))
}
