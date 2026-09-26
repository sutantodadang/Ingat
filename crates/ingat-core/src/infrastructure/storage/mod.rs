//! Storage adapters for Ingat.
//!
//! * [`sled_store`] — legacy embedded sled-backed vector store
//!   (feature `sled-store`).
//! * [`sqlite_store`] — concurrent SQLite-backed vector store
//!   (feature `sqlite-store`).
//! * [`export`] — offline JSONL export of a stopped sled store
//!   (feature `legacy-export`).

#[cfg(any(feature = "sled-store", feature = "sqlite-store"))]
pub(crate) mod similarity;

#[cfg(feature = "legacy-export")]
pub mod export;
#[cfg(feature = "sled-store")]
pub mod sled_store;
#[cfg(feature = "sqlite-store")]
pub mod sqlite_store;

#[cfg(feature = "legacy-export")]
pub use export::export_legacy_jsonl;
#[cfg(feature = "sled-store")]
pub use sled_store::SledVectorStore;
#[cfg(feature = "sqlite-store")]
pub use sqlite_store::SqliteVectorStore;
