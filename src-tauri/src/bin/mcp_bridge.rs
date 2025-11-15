#![allow(clippy::expect_used)]

#[cfg(feature = "mcp-server")]
use ingat_lib::run_mcp_bridge;

#[cfg(feature = "mcp-server")]
use tauri::async_runtime;

#[cfg(feature = "mcp-server")]
fn main() {
    if let Err(err) = async_runtime::block_on(run_mcp_bridge(None)) {
        eprintln!("[ingat::mcp-bridge] runtime failed: {err:?}");
        std::process::exit(1);
    }
}

#[cfg(not(feature = "mcp-server"))]
fn main() {
    eprintln!(
        "[ingat::mcp-bridge] build with `--features mcp-server` to enable the MCP bridge binary."
    );
    std::process::exit(1);
}
