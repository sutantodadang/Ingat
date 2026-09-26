//! Domain layer re-exported from `ingat-core`.
//!
//! Records, value objects and errors live in the headless core crate. This shim
//! keeps the desktop/server call-sites on their existing `crate::domain` paths.

pub use ingat_core::domain::*;
