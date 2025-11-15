#![allow(clippy::expect_used)]

#[cfg(feature = "mcp-server")]
use ingat_lib::run_mcp_stdio;

#[cfg(feature = "mcp-server")]
use tauri::async_runtime;

/// Standalone MCP bridge using stdio transport (stdin/stdout).
///
/// This binary is designed for MCP clients that spawn a process and communicate
/// via standard input/output, such as:
/// - VS Code MCP extension
/// - Cursor IDE
/// - Windsurf IDE
/// - Sublime Text with MCP plugins
///
/// Unlike the SSE-based `mcp-bridge`, this binary reads JSON-RPC messages from
/// stdin and writes responses to stdout, making it compatible with process-based
/// MCP clients.
///
/// # Usage
///
/// Configure your IDE to spawn this binary:
/// # Example Configuration
///
/// ```json
/// {
///   "mcpServers": {
///     "ingat": {
///       "command": "/path/to/mcp-stdio",
///       "args": []
///     }
///   }
/// }
/// ```
///
/// # Environment Variables
///
/// - `INGAT_LOG`: Set logging level (trace, debug, info, warn, error)
/// - `INGAT_DATA_DIR`: Override data directory location
///
#[cfg(feature = "mcp-server")]
fn main() {
    // Block on the async runtime
    if let Err(err) = async_runtime::block_on(run_mcp_stdio()) {
        eprintln!("[ingat::mcp-stdio] Runtime failed: {err:?}");
        std::process::exit(1);
    }
}

#[cfg(not(feature = "mcp-server"))]
fn main() {
    eprintln!(
        "[ingat::mcp-stdio] Build with `--features mcp-server` to enable the MCP stdio bridge."
    );
    eprintln!("Example: cargo build --release --bin mcp-stdio --features mcp-server");
    std::process::exit(1);
}
