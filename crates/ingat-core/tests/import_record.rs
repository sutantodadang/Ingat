//! `ContextService::import_record` scope/metadata preservation and durability.

#![cfg(feature = "sqlite-store")]

use std::sync::Arc;

use ingat_core::application::services::{ContextService, ServiceConfig, VectorStore};
use ingat_core::domain::{ContextEmbedding, ContextKind, ContextRecord, MemoryScope};
use ingat_core::infrastructure::{Sha256EmbedEngine, SqliteVectorStore};

const MODEL: &str = "ingat/simple-sha256-v1";

fn fixture(id: uuid::Uuid, scope: MemoryScope) -> ContextRecord {
    ContextRecord {
        id,
        project: "kode".to_string(),
        ide: "legacy-ide".to_string(),
        file_path: Some("legacy/file.rs".to_string()),
        language: Some("rust".to_string()),
        summary: "legacy summary".to_string(),
        body: "shared body".to_string(),
        tags: vec!["legacy".to_string()],
        kind: ContextKind::FixHistory,
        embedding: ContextEmbedding::new("ingat/simple-hash", vec![0.1, 0.2, 0.3]),
        created_at: chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
        scope,
        author: Some("alice".to_string()),
        provenance: Some("user".to_string()),
    }
}

#[test]
fn import_record_preserves_scope_metadata_and_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("memory.sqlite");

    let store = Arc::new(SqliteVectorStore::open(&db).unwrap());
    let embedder = Arc::new(Sha256EmbedEngine::try_new(MODEL, 256).unwrap());
    let service = ContextService::new(embedder, store.clone(), ServiceConfig::new(MODEL, 8));

    let personal_id = uuid::Uuid::new_v4();
    let team_id = uuid::Uuid::new_v4();
    let personal = fixture(personal_id, MemoryScope::Personal);
    let team = fixture(team_id, MemoryScope::Team);

    assert!(service.import_record(&personal).unwrap());
    assert!(service.import_record(&team).unwrap());
    // Re-import must skip without mutating.
    assert!(!service.import_record(&personal).unwrap());
    assert_eq!(service.record_count().unwrap(), 2);

    let stored_personal = store.get(&personal_id).unwrap().expect("personal present");
    assert_eq!(stored_personal.scope, MemoryScope::Personal);
    assert_eq!(stored_personal.author.as_deref(), Some("alice"));
    assert_eq!(stored_personal.provenance.as_deref(), Some("user"));
    assert_eq!(stored_personal.created_at, personal.created_at);
    assert_eq!(stored_personal.body, "shared body");
    assert_eq!(stored_personal.tags, vec!["legacy".to_string()]);
    // Re-embedded with the target model, not relabelled from the source vector.
    assert_eq!(stored_personal.embedding.model, MODEL);
    assert_eq!(stored_personal.embedding.dims(), 256);

    let stored_team = store.get(&team_id).unwrap().expect("team present");
    assert_eq!(stored_team.scope, MemoryScope::Team);

    // Durable commit survives a fresh connection.
    drop(service);
    drop(store);
    let reopened = SqliteVectorStore::open(&db).unwrap();
    assert_eq!(reopened.count().unwrap(), 2);
}
