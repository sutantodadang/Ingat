# ingat-core

Headless, **Tauri-free** core of the [Ingat](../README.md) context memory
system. It is meant to be embedded **in-process** by consumers such as Kode —
no GUI, no HTTP service, no MCP transport, no OS data-path discovery.

The crate is a straight extraction of Ingat's existing retrieval path; it does
not fork or rewrite it. The desktop app (`src-tauri`) depends on this crate and
re-exports it, keeping the desktop/MCP/HTTP transports and Tauri bootstrap out
of the core.

## Public paths

| Path | Contents |
| --- | --- |
| `ingat_core::domain` | `ContextRecord`, `ContextSummary`, `ContextKind`, `MemoryScope`, `QueryFilters`, `RetrievalQuery`, `ContextEmbedding`, `DomainError` |
| `ingat_core::application::dtos` | `IngestContextRequest`, `SearchRequest`, `SearchResponse`, `SummaryListResponse`, `HealthStatusResponse`, `WireMemoryEntry`, `ImportResponse`, … |
| `ingat_core::application::services` | `ContextService`, the `ContextApi`, `VectorStore` and `EmbeddingEngine` traits, `ServiceConfig` |
| `ingat_core::infrastructure::storage` | `SledVectorStore` (feature `sled-store`) |
| `ingat_core::infrastructure::embeddings` | `SimpleEmbedEngine`, `NoOpEmbeddingEngine`, `FastEmbedEngine` (feature `fastembed-engine`) |

Constructors accept explicit options. The core never discovers OS data paths,
probes or starts services, initialises a global logger, prints, or installs an
application runtime — that wiring stays in the desktop/server crate.

## Features

| Feature | Default | Description |
| --- | --- | --- |
| `sled-store` | ✅ | Legacy sled-backed `VectorStore` adapter |
| `schema` | ❌ | Derive `schemars::JsonSchema` for the wire DTOs (used by the MCP server) |
| `fastembed-engine` | ❌ | Optional `fastembed`/ONNX embedding engine |
| `sqlite-store` | ❌ | Reserved for the storage follow-up (adds no adapter or dependency yet) |

The minimum headless build (`--no-default-features`) pulls in no Tauri, plugins,
`axum`, `rmcp` or `fastembed`:

```bash
cargo tree -p ingat-core --no-default-features
cargo test -p ingat-core
```

## Minimal consumer example

Add a pinned dependency (see the merged commit SHA for the revision Kode pins):

```toml
[dependencies]
ingat-core = { git = "https://github.com/sutantodadang/Ingat", rev = "<merged-commit-sha>", default-features = false, features = ["sled-store"] }
```

Then construct the existing core types directly:

```rust
use std::sync::Arc;

use ingat_core::application::dtos::{IngestContextRequest, SearchRequest};
use ingat_core::application::services::{ContextService, ServiceConfig};
use ingat_core::domain::{ContextKind, MemoryScope};
use ingat_core::infrastructure::{SimpleEmbedEngine, SledVectorStore};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Explicit path only — the core never discovers the OS data directory.
    let store = Arc::new(SledVectorStore::open("./ingat-store")?);
    let embedder = Arc::new(SimpleEmbedEngine::try_new("ingat/simple-hash", 256)?);
    let service = ContextService::new(embedder, store, ServiceConfig::new("ingat/simple-hash", 8));

    let summary = service.ingest(IngestContextRequest {
        project: "kode".into(),
        ide: "kode".into(),
        file_path: None,
        language: Some("rust".into()),
        summary: "headless consumer example".into(),
        body: "constructing the existing service/core types".into(),
        tags: vec!["kode".into()],
        kind: ContextKind::CodeSnippet,
        scope: MemoryScope::Personal,
    })?;

    let response = service.search(SearchRequest {
        prompt: "headless consumer".into(),
        filters: Default::default(),
        limit: 5,
    })?;

    println!("{} hit(s); top id = {}", response.results.len(), summary.id);
    Ok(())
}
```

Consumers that only need the abstraction can program against
`ingat_core::application::services::ContextApi` (`Arc<dyn ContextApi>`) instead
of the concrete `ContextService`.

## License

Apache-2.0, matching the repository [LICENSE](../../LICENSE).
