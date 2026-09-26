//! Offline export of a stopped sled store to JSONL (`legacy-export`).
//!
//! The export acquires the normal sled ownership lock, so it fails when the
//! store is active/locked instead of copying a live database. Output is written
//! to a sibling temporary file, flushed and synced, then renamed over the
//! destination; the source store and any pre-existing destination are left
//! untouched on failure. No desktop/server startup is involved.

use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::application::dtos::LegacyExportLine;
use crate::application::services::VectorStore;
use crate::domain::{ContextRecord, DomainError};

use super::sled_store::SledVectorStore;

/// Reads every current and legacy pre-scope record from the sled store at
/// `store_path` and writes each as a `{"v":1,"record":...}` JSON line to
/// `out_path`. Returns the number of exported records.
pub fn export_legacy_jsonl(
    store_path: impl AsRef<Path>,
    out_path: impl AsRef<Path>,
) -> Result<u64, DomainError> {
    // Acquiring the store takes the normal sled ownership lock, so an active or
    // otherwise locked store is refused here.
    let store = SledVectorStore::open(store_path.as_ref())?;
    let records = store.all()?;

    let out = out_path.as_ref();
    let file_name = out
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| DomainError::validation("output path must have a file name"))?;
    let tmp = out.with_file_name(format!("{file_name}.tmp"));

    match write_jsonl(&tmp, &records) {
        Ok(count) => {
            std::fs::rename(&tmp, out).map_err(|err| {
                let _ = std::fs::remove_file(&tmp);
                DomainError::storage(format!("failed to replace output: {err}"))
            })?;
            Ok(count)
        }
        Err(err) => {
            let _ = std::fs::remove_file(&tmp);
            Err(err)
        }
    }
}

fn write_jsonl(path: &Path, records: &[ContextRecord]) -> Result<u64, DomainError> {
    let mut file = File::create(path)
        .map_err(|err| DomainError::storage(format!("failed to create export file: {err}")))?;

    let mut count = 0u64;
    for record in records {
        let line = LegacyExportLine {
            v: 1,
            record: record.clone(),
        };
        // Errors carry only the serde failure, never record content.
        serde_json::to_writer(&mut file, &line).map_err(|err| {
            DomainError::storage(format!("failed to serialize export line: {err}"))
        })?;
        file.write_all(b"\n")
            .map_err(|err| DomainError::storage(format!("failed to write export line: {err}")))?;
        count += 1;
    }

    file.flush()
        .map_err(|err| DomainError::storage(format!("failed to flush export: {err}")))?;
    file.sync_all()
        .map_err(|err| DomainError::storage(format!("failed to sync export: {err}")))?;
    Ok(count)
}
