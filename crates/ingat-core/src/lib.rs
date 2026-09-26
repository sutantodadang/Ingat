//! # ingat-core
//!
//! Headless, Tauri-free core of the Ingat context memory system.
//!
//! The crate exposes the domain model, the application services (including the
//! [`application::services::ContextService`] orchestrator and the
//! [`application::services::ContextApi`] / [`application::services::VectorStore`]
//! / [`application::services::EmbeddingEngine`] traits) and the concrete
//! storage/embedding adapters under stable module paths:
//!
//! * [`domain`]
//! * [`application::dtos`], [`application::services`]
//! * [`infrastructure::storage`], [`infrastructure::embeddings`]
//!
//! Constructors take explicit options only: the core never discovers OS data
//! paths, probes or starts services, initialises a global logger, prints to
//! stdout/stderr, or installs an application runtime. That wiring lives in the
//! desktop/server crate.

pub mod application;
pub mod domain;
pub mod infrastructure;
