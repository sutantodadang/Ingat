# Ingat

**A local-first context memory system for developers with multi-client support**

Ingat is a Tauri-based desktop application that stores code snippets, fixes, discussions, and other development context locally on your machine. It provides semantic search capabilities and integrates with IDEs via the Model Context Protocol (MCP).

**🎉 New: Multi-Client Remote Mode!** Use the UI and multiple IDEs simultaneously without database conflicts.

---

## ✨ Features

- 🔒 **Privacy-First**: All data stored locally, no cloud services
- 🧠 **Semantic Search**: Find context by meaning, not just keywords
- 🚀 **Multi-Client Support**: Use UI + multiple IDEs simultaneously
- 🔌 **Universal MCP Integration**: Works with all major IDEs via stdio and HTTP/SSE transports
- ⚡ **Multiple Embedding Backends**: Choose between lightweight deterministic hash or high-quality FastEmbed
- 🎨 **Modern UI**: Built with React, TypeScript, and Tailwind CSS
- 🦀 **Rust Backend**: Fast, safe, and reliable

---

## 🚀 Quick Start

### Option 1: Use the Helper Script (Recommended)

**Windows:**

```powershell
# Clone and navigate
git clone https://github.com/sutantodadang/Ingat.git
cd Ingat

# Install dependencies
bun install

# Start everything (service + UI)
.\start-with-service.ps1
```

This automatically:

1. ✅ Builds the backend service if needed
2. ✅ Starts `mcp_service` (holds database lock)
3. ✅ Starts the UI in remote mode
4. ✅ Enables simultaneous IDE connections

### Option 2: Manual Setup

```bash
# 1. Install dependencies
bun install

# 2. Build the service
cd src-tauri
cargo build --release --bin mcp_service --features mcp-server,tauri-plugin

# 3. Start the service (keep this terminal open)
./target/release/mcp_service

# 4. In a new terminal, start the UI
cd ..
bun run dev
```

**Then connect your IDE(s)** - see [SETUP_GUIDE.md](./SETUP_GUIDE.md)

### Recommended for Windsurf (Cascade)

If you already keep `mcp-service` running as a background process, configure Windsurf to spawn `mcp-stdio`.

**What happens:**

- `mcp-stdio` detects the running `mcp-service` and automatically uses **REMOTE MODE**
- All `ingest_context` / `search_contexts` calls are proxied to the service over HTTP
- This avoids local DB locks and makes “store / retrieve context” reliable when multiple clients are open

See [SETUP_GUIDE.md](./SETUP_GUIDE.md) → **Multi-Client Mode** and **Windsurf**.

---

## 📖 Documentation

### 🌟 Start Here

- **[SETUP_GUIDE.md](./SETUP_GUIDE.md)** - **Complete setup guide for all users** ⭐
- **[START_HERE.md](./START_HERE.md)** - Quick troubleshooting and startup guide

### IDE Integration

- **[IDE_MCP_SETUP.md](./IDE_MCP_SETUP.md)** - Setup for VS Code, Cursor, Windsurf, Sublime, Zed, Claude
- **[VS_CODE_MCP_SETUP.md](./VS_CODE_MCP_SETUP.md)** - Detailed VS Code/Cursor/Windsurf guide

### Multi-Client Usage

- **[QUICK_FIX.md](./QUICK_FIX.md)** - Fix database lock conflicts
- **[MULTI_CLIENT_USAGE.md](./MULTI_CLIENT_USAGE.md)** - Use Ingat across multiple IDEs
- **[UNIFIED_SERVICE_SETUP.md](./UNIFIED_SERVICE_SETUP.md)** - Production backend service guide
- **[docs/REMOTE_MODE.md](./docs/REMOTE_MODE.md)** - Technical details on remote mode

### Technical Documentation

- **[MCP_INTEGRATION.md](./MCP_INTEGRATION.md)** - Architecture and API reference
- **[docs/IMPLEMENTATION_SUMMARY.md](./docs/IMPLEMENTATION_SUMMARY.md)** - Implementation details
- **[docs/ARCHITECTURE_DIAGRAMS.md](./docs/ARCHITECTURE_DIAGRAMS.md)** - Visual architecture guides

---

## 🎯 MCP Binaries

Ingat provides three MCP server binaries for different use cases:

| Binary               | Transport       | Best For                            | Setup Guide                            |
| -------------------- | --------------- | ----------------------------------- | -------------------------------------- |
| **`mcp_stdio`**      | stdin/stdout    | VS Code, Cursor, Windsurf, Sublime  | [IDE_MCP_SETUP.md](./IDE_MCP_SETUP.md) |
| **`mcp_bridge`**     | HTTP/SSE        | Zed, Claude Desktop                 | [IDE_MCP_SETUP.md](./IDE_MCP_SETUP.md) |
| **`mcp_service`** 🆕 | HTTP/REST + SSE | **Multi-client simultaneous usage** | [SETUP_GUIDE.md](./SETUP_GUIDE.md)     |

**Tip (Windsurf/Cascade):** run `mcp-service` in the background and point Windsurf to `mcp-stdio`. Ingat will proxy calls to the service automatically.

### Building Binaries

```bash
cd src-tauri

# Build all MCP binaries
cargo build --release --bin mcp_stdio --features mcp-server
cargo build --release --bin mcp_bridge --features mcp-server
cargo build --release --bin mcp_service --features mcp-server,tauri-plugin
```

Binaries will be in: `src-tauri/target/release/`

---

## 🏗️ Architecture

### Single Client Mode (Default)

```
┌─────────────┐
│  Tauri UI   │─────► Local DB ✓
└─────────────┘
```

### Multi-Client Mode (Remote Mode) 🆕

```
┌─────────────┐
│  Tauri UI   │────┐
└─────────────┘    │
                   ├───► mcp-service ───► Local DB ✓
┌─────────────┐    │     (Single lock)
│  VS Code    │────┤
└─────────────┘    │
                   │
┌─────────────┐    │
│    Zed      │────┘
└─────────────┘
```

**How it works:**

- `mcp-service` holds the exclusive database lock
- All clients (UI, IDEs) connect via HTTP
- No database lock conflicts
- Automatic service detection on startup

See [docs/REMOTE_MODE.md](./docs/REMOTE_MODE.md) for details.

---

## 🎓 Supported IDEs

| IDE                | Status          | Transport | Configuration                |
| ------------------ | --------------- | --------- | ---------------------------- |
| **VS Code**        | ✅ Full Support | stdio     | `.vscode/settings.json`      |
| **Cursor**         | ✅ Full Support | stdio     | `.cursor/mcp.json`           |
| **Windsurf**       | ✅ Full Support | stdio     | `.windsurf/mcp.json`         |
| **Sublime Text**   | ✅ Full Support | stdio     | Codeium config               |
| **Zed**            | ✅ Full Support | SSE       | `settings.json`              |
| **Claude Desktop** | ✅ Full Support | SSE       | `claude_desktop_config.json` |

**See [SETUP_GUIDE.md](./SETUP_GUIDE.md) for complete setup instructions.**

---

## ⚙️ Configuration

### Environment Variables

```bash
# Service configuration
export INGAT_SERVICE_HOST="127.0.0.1"  # Default: 127.0.0.1
export INGAT_SERVICE_PORT="3200"        # Default: 3200
export INGAT_DATA_DIR="/custom/path"    # Optional: custom data directory
export INGAT_LOG="info"                 # Logging: trace, debug, info, warn, error
```

**Note:** when a running `mcp-service` is detected, Ingat uses remote mode and proxies context operations to the service. In that mode, changing the embedding backend from the client is not supported.

### Data Storage

Default data locations:

- **Windows:** `%APPDATA%\ingat\Ingat\data`
- **macOS:** `~/Library/Application Support/ingat/Ingat/data`
- **Linux:** `~/.config/ingat/Ingat/data`

---

## 🔧 Development

### Prerequisites

- [Node.js](https://nodejs.org/) (v18+) or [Bun](https://bun.sh/)
- [Rust](https://rustup.rs/) (latest stable)
- [Tauri Prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites)

### Run in Development

```bash
# Install dependencies
bun install

# Run Tauri app in dev mode
bun run tauri dev

# Or with all features
bun run tauri dev --features fastembed-engine,mcp-server
```

### Build for Production

```bash
# Build installer with all binaries
bun run tauri build --features mcp-server,tauri-plugin

# Or with FastEmbed
bun run tauri build --features fastembed-engine,mcp-server,tauri-plugin
```

The installer will include:

- Tauri UI application
- `mcp-stdio.exe`
- `mcp-bridge.exe`
- `mcp-service.exe`

---

## 🧪 Testing

```bash
# Rust tests (run from the workspace root)
cargo test                 # all crates
cargo test -p ingat-core   # headless core only (no desktop system packages)

# Check compilation
cargo check --workspace --all-features

# Lint
cargo clippy --workspace --all-features

# Format
cargo fmt
```

---

## 🧩 ingat-core (headless)

The retrieval path is also available as a Tauri-free library crate,
[`crates/ingat-core`](./crates/ingat-core/README.md), for in-process consumers
such as Kode. It exposes the same domain, application services and
storage/embedding adapters under stable paths (`domain`,
`application::{dtos,services}`, `infrastructure::{storage,embeddings}`) with no
GUI, HTTP service, MCP transport or OS data-path discovery. The desktop crate
depends on it and re-exports it, so existing call-sites are unchanged.

```bash
# Headless build: no Tauri/plugins/axum/rmcp/fastembed in the dependency tree
cargo tree -p ingat-core --no-default-features
cargo test -p ingat-core
```

| Core feature       | Default | Description                                            |
| ------------------ | ------- | ------------------------------------------------------ |
| `sled-store`       | ✅      | Legacy sled-backed `VectorStore` adapter               |
| `sqlite-store`     | ❌      | Concurrent SQLite adapter + `open_embedded`            |
| `legacy-export`    | ❌      | Offline `ingat_export` CLI to JSONL (implies sled)     |
| `schema`           | ❌      | Derive `schemars::JsonSchema` for the wire DTOs        |
| `fastembed-engine` | ❌      | Optional `fastembed`/ONNX embedding engine             |

The `sqlite-store` feature adds a concurrent store safe for multiple processes
(`open_embedded` + the stable `ingat/simple-sha256-v1` hash model), and
`legacy-export` adds an `ingat_export` binary that migrates a stopped sled store
to JSONL. See [crates/ingat-core/README.md](./crates/ingat-core/README.md) for
the API, feature flags and a minimal consumer example.

---

## 📦 Feature Flags

| Flag               | Default | Description                               |
| ------------------ | ------- | ----------------------------------------- |
| `simple-embed`     | ✅      | Lightweight deterministic hash embeddings |
| `fastembed-engine` | ❌      | High-quality semantic embeddings (ONNX)   |
| `mcp-server`       | ✅      | Model Context Protocol server support     |
| `tauri-plugin`     | ✅      | Required for `mcp-service` HTTP server    |

---

## 🎯 Use Cases

### Individual Developer

- Store code snippets and solutions
- Semantic search across your knowledge base
- IDE integration for quick context access

### Team Usage

- Run `mcp-service` on a shared server
- Team members connect their IDEs remotely
- Centralized context repository
- See [UNIFIED_SERVICE_SETUP.md](./UNIFIED_SERVICE_SETUP.md)

### Multi-Tool Workflow

- Use UI for browsing and management
- Use VS Code for coding
- Use Claude Desktop for AI assistance
- All accessing the same data simultaneously

---

## 🚨 Troubleshooting

### Database Lock Errors

**Symptom:** "could not acquire lock" error

**Solution:** Use multi-client mode with `mcp-service`

```powershell
# Stop everything
Get-Process | Where-Object {$_.ProcessName -match "ingat|mcp"} | Stop-Process -Force

# Start in correct order
.\start-with-service.ps1
```

**See [QUICK_FIX.md](./QUICK_FIX.md) for detailed troubleshooting.**

### Service Won't Start

**Check:**

1. Is another instance running? `curl http://localhost:3200/health`
2. Is the UI holding the DB lock? Close it first
3. Port 3200 available? `netstat -an | findstr 3200`

### IDE Connection Fails

**Check:**

1. Service is running: `curl http://localhost:3200/health`
2. Binary path is correct in IDE config
3. Binaries are up to date: rebuild with `cargo build --release`

**See [START_HERE.md](./START_HERE.md) for complete troubleshooting guide.**

---

## 🤝 Contributing

Contributions are welcome! Please:

1. Follow clean architecture principles
2. Use feature flags for optional dependencies
3. Add documentation for new features
4. Run `cargo clippy` and `cargo fmt` before committing
5. Update relevant documentation files
6. Write tests for new functionality

### Project Structure

```
ingat/
├── Cargo.toml              # Workspace root (members: src-tauri, crates/ingat-core)
├── crates/
│   └── ingat-core/         # Headless, Tauri-free core (domain/application/infrastructure)
├── src/                    # Frontend (React + TypeScript)
├── src-tauri/              # Desktop/MCP/HTTP crate (Tauri app + binaries)
│   ├── src/
│   │   ├── application/    # Re-exports `ingat-core` (dto/service paths preserved)
│   │   ├── domain/         # Re-exports `ingat-core`
│   │   ├── infrastructure/ # Re-exports core adapters + HTTP client
│   │   ├── interfaces/     # MCP servers
│   │   ├── bin/            # Standalone binaries
│   │   └── settings.rs, power_manager.rs, service_manager.rs
│   └── Cargo.toml
├── docs/                   # Additional documentation
└── scripts/                # Helper scripts
```

---

## 📄 License

[Your License Here]

---

## 🙏 Acknowledgments

- [Tauri](https://tauri.app/) - Desktop app framework
- [rmcp](https://github.com/QuantumBear/rmcp) - Rust MCP SDK
- [FastEmbed](https://github.com/Anush008/fastembed-rs) - Embedding library
- [Sled](https://github.com/spacejam/sled) - Embedded database
- [Model Context Protocol](https://modelcontextprotocol.io/) - MCP specification

---

## 📞 Support & Community

- 📖 **Setup Guide:** [SETUP_GUIDE.md](./SETUP_GUIDE.md)
- 🐛 **Report Issues:** [GitHub Issues](https://github.com/sutantodadang/Ingat/issues)
- 💬 **Discussions:** [GitHub Discussions](https://github.com/sutantodadang/Ingat/discussions)
- ❓ **Quick Fix:** [QUICK_FIX.md](./QUICK_FIX.md)

---

## 🌟 What's New

### v0.1.0 - Initial Release

- ✨ Remote mode for simultaneous UI + IDE usage
- 🔧 `mcp-service` unified backend
- 📖 Comprehensive setup guides
- 🚀 Helper scripts for easy startup
- 🔍 Automatic service detection

See [docs/IMPLEMENTATION_SUMMARY.md](./docs/IMPLEMENTATION_SUMMARY.md) for technical details.

---

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- [Zed](https://zed.dev/) with native MCP support

---

**Get Started:** Read [SETUP_GUIDE.md](./SETUP_GUIDE.md) for complete setup instructions! 🚀
