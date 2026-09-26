//! Headless fixture: ingest then search through the core service and the sled
//! adapter, with no listener, child process or installed user store. The
//! fixture lives in a temporary directory.

#![cfg(feature = "sled-store")]

use std::sync::Arc;

use ingat_core::application::dtos::{IngestContextRequest, SearchRequest};
use ingat_core::application::services::{ContextService, ServiceConfig};
use ingat_core::domain::{ContextKind, MemoryScope};
use ingat_core::infrastructure::{SimpleEmbedEngine, SledVectorStore};

const MODEL: &str = "ingat/simple-hash";
const BODY: &str = "the headless core round trips uuid, project and content";

#[test]
fn headless_ingest_search_round_trip() {
    let dir = tempfile::tempdir().expect("temporary fixture directory");
    let store = Arc::new(SledVectorStore::open(dir.path()).expect("open temporary sled store"));
    let embedder =
        Arc::new(SimpleEmbedEngine::try_new(MODEL, 256).expect("construct simple embedder"));
    let service = ContextService::new(embedder, store, ServiceConfig::new(MODEL, 8));

    let summary = service
        .ingest(IngestContextRequest {
            project: "kode".into(),
            ide: "vscode".into(),
            file_path: Some("src/lib.rs".into()),
            language: Some("rust".into()),
            summary: "round trip fixture".into(),
            body: BODY.into(),
            tags: vec!["fixture".into()],
            kind: ContextKind::CodeSnippet,
            scope: MemoryScope::Personal,
        })
        .expect("ingest fixture");

    let response = service
        .search(SearchRequest {
            prompt: "headless core round trip".into(),
            filters: Default::default(),
            limit: 5,
        })
        .expect("search fixture");

    assert_eq!(response.results.len(), 1, "fixture should be retrievable");
    let hit = &response.results[0];
    assert_eq!(hit.id, summary.id, "UUID must round-trip");
    assert_eq!(hit.project, "kode", "project must round-trip");
    assert_eq!(hit.body, BODY, "content must round-trip");
}
