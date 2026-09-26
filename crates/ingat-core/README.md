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
| `ingat_core::application::dtos` | `IngestContextRequest`, `SearchRequest`, `SearchResponse`, `SummaryListResponse`, `HealthStatusResponse`, `WireMemoryEntry`, `ImportResponse`, `LegacyExportLine`, … |
| `ingat_core::application::services` | `ContextService`, the `ContextApi`, `VectorStore` and `EmbeddingEngine` traits, `ServiceConfig` |
| `ingat_core::application::embedded` | `open_embedded`, `EmbeddedOptions` (feature `sqlite-store`) |
| `ingat_core::infrastructure::storage` | `SledVectorStore` (feature `sled-store`), `SqliteVectorStore` (feature `sqlite-store`), `export_legacy_jsonl` (feature `legacy-export`) |
| `ingat_core::infrastructure::embeddings` | `SimpleEmbedEngine`, `Sha256EmbedEngine`, `NoOpEmbeddingEngine`, `FastEmbedEngine` (feature `fastembed-engine`) |

Constructors accept explicit options. The core never discovers OS data paths,
probes or starts services, initialises a global logger, prints, or installs an
application runtime — that wiring stays in the desktop/server crate.

## Features

| Feature | Default | Description |
| --- | --- | --- |
| `sled-store` | ✅ | Legacy sled-backed `VectorStore` adapter |
| `sqlite-store` | ❌ | Concurrent SQLite (rusqlite bundled) `VectorStore` adapter + `open_embedded` |
| `legacy-export` | ❌ | Offline `ingat_export` CLI to JSONL (implies `sled-store`) |
| `schema` | ❌ | Derive `schemars::JsonSchema` for the wire DTOs (used by the MCP server) |
| `fastembed-engine` | ❌ | Optional `fastembed`/ONNX embedding engine |

The minimum headless build (`--no-default-features`) pulls in no Tauri, plugins,
`axum`, `rmcp`, `fastembed`, `sled`, `bincode`, `rusqlite` or `serde_json`:

```bash
cargo tree -p ingat-core --no-default-features
cargo test -p ingat-core
cargo test -p ingat-core --features sqlite-store,legacy-export
```

## Concurrent SQLite memory (`sqlite-store`)

`SqliteVectorStore::open(path)` opens (or creates) a database safe to share
between multiple OS processes:

- Bundled SQLite is asserted `>= 3.51.3` (the WAL-reset fix); a too-old bundle
  is a hard error, never silently accepted.
- `journal_mode = WAL`, `synchronous = FULL`, `busy_timeout = 5000 ms`,
  `PRAGMA user_version = 1` (a newer schema version is rejected).
- Table `contexts(id, project, kind, scope, created_at, model, dimensions,
  record_json)` with indexes on `(project, created_at)` and
  `(project, kind)`. The full `ContextRecord` JSON is preserved.
- One mutex-protected connection per instance; writers serialise through
  SQLite. No transaction is held while embeddings are computed.
- `VectorStore::insert_if_absent` is atomic
  (`INSERT ... ON CONFLICT(id) DO NOTHING`) and returns `true` only for this
  call's actual insertion, so racing importers report correct counts.

`open_embedded(EmbeddedOptions { database_path })` wires the store to the stable
`ingat/simple-sha256-v1` hash model (256 dims, retrieval limit 8).

## Stable model (`ingat/simple-sha256-v1`)

The existing `ingat/simple-hash` `SimpleEmbedEngine` keeps its legacy
(`AHasher`) behavior. The new `Sha256EmbedEngine` derives each token slot from
SHA-256 — first eight digest bytes as a little-endian `u64`, `slot = value %
dimensions` — with the same tokenisation, counts and L2 normalisation. Vectors
are therefore identical across independently started processes and restarts.
Imports always **re-embed** with the target model; stored vectors are never
relabelled or reused.

## Legacy migration (`legacy-export`)

```bash
cargo run -p ingat-core --features legacy-export --bin ingat_export -- \
  --store ./old-sled-store --out ./memory.jsonl
```

- Acquires the normal sled ownership lock: a live/active store is refused
  (the export errors instead of copying a live database).
- Reads both current and pre-scope legacy records.
- Writes `{"v":1,"record":…}` per line to a sibling temporary file, flushes and
  syncs it, then renames over the destination. On failure the source and any
  existing output are left untouched.
- Runs without any desktop/server startup and prints only counts.

`ContextService::import_record(&record)` preserves the UUID, personal/team
scope, project, kind, tags, author, provenance, body, summary and timestamp,
recomputes the embedding and skips an existing UUID without overwriting it.
Distinct UUIDs with identical text and different scopes stay distinct.

## Minimal consumer example

Add a pinned dependency (see the merged commit SHA for the revision Kode pins):

```toml
[dependencies]
ingat-core = { git = "https://github.com/sutantodadang/Ingat", rev = "<merged-commit-sha>", default-features = false, features = ["sqlite-store"] }
```

```rust
use ingat_core::application::dtos::{IngestContextRequest, SearchRequest};
use ingat_core::domain::{ContextKind, MemoryScope};
use ingat_core::{open_embedded, EmbeddedOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Explicit path only — the core never discovers the OS data directory.
    let service = open_embedded(EmbeddedOptions {
        database_path: "./kode-memory.sqlite".into(),
    })?;

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
        limit: 8,
    })?;

    println!(
        "{} hit(s); count = {}; top id = {}",
        response.results.len(),
        service.record_count()?,
        summary.id
    );
    Ok(())
}
```

Consumers that only need the abstraction can program against
`ingat_core::application::services::ContextApi` (`Arc<dyn ContextApi>`) instead
of the concrete `ContextService`.

## Process-test evidence

`cargo test -p ingat-core --features sqlite-store` spawns the
`ingat_test_driver` binary as **real OS processes** to prove:

- two processes write 100 unique records each → 200 records after reopen and
  `integrity_check = ok`;
- two processes import the same UUID → the sum of reported insertions is
  exactly 1;
- SHA-256 embeddings for Indonesian/English/Unicode fixtures are byte-identical
  across independent processes;
- a blocked writer (>5 s busy owner) returns an error instead of faking
  success.

## License

Apache-2.0, matching the repository [LICENSE](../../LICENSE).
