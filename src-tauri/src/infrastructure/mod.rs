//! Infrastructure layer for the desktop/server crate.
//!
//! Concrete storage and embedding adapters now live in `ingat-core`; this
//! module re-exports them so existing call-sites keep the same paths, and adds
//! the desktop-only HTTP client used for remote `mcp-service` mode.

pub use ingat_core::infrastructure::*;

pub mod http_client;
pub use http_client::{check_service_availability, RemoteContextClient, RemoteVectorStore};
