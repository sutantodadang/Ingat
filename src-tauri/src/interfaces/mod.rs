#![allow(dead_code)]
// Interfaces exposed by optional adapters (e.g., MCP servers or bridges).
//
// Each submodule should be feature-gated by the capability it implements.
#[cfg(feature = "mcp-server")]
pub mod mcp;
