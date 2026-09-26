//! Test/evidence driver for the SQLite embedded store. Spawned as a real OS
//! process by the integration tests to prove cross-process concurrency and
//! stable embeddings. Not part of the public API surface.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use ingat_core::application::dtos::IngestContextRequest;
use ingat_core::application::services::{EmbeddingEngine, VectorStore};
use ingat_core::application::{open_embedded, EmbeddedOptions, SIMPLE_SHA256_MODEL};
use ingat_core::domain::{ContextEmbedding, ContextKind, ContextRecord, MemoryScope};
use ingat_core::infrastructure::{Sha256EmbedEngine, SqliteVectorStore};

fn usage() -> &'static str {
    "usage: ingat_test_driver <write|count|verify|import|embed> [--key value]..."
}

fn parse(opts: &[String]) -> Result<HashMap<String, String>, String> {
    let mut map = HashMap::new();
    let mut i = 0;
    while i < opts.len() {
        let key = opts[i].trim_start_matches("--").to_string();
        let value = opts
            .get(i + 1)
            .ok_or_else(|| format!("missing value for {key}"))?
            .clone();
        map.insert(key, value);
        i += 2;
    }
    Ok(map)
}

fn opt<'a>(opts: &'a HashMap<String, String>, key: &str) -> Result<&'a str, String> {
    opts.get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("missing --{key}"))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("{}", usage());
        return ExitCode::FAILURE;
    }

    let opts = match parse(&args[1..]) {
        Ok(opts) => opts,
        Err(err) => {
            eprintln!("ingat_test_driver: {err}");
            return ExitCode::FAILURE;
        }
    };

    match run(&args[0], &opts) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("ingat_test_driver: {err}");
            ExitCode::FAILURE
        }
    }
}

fn open(db: &str) -> Result<ingat_core::application::services::ContextService, String> {
    open_embedded(EmbeddedOptions {
        database_path: PathBuf::from(db),
    })
    .map_err(|err| err.to_string())
}

fn run(command: &str, opts: &HashMap<String, String>) -> Result<(), String> {
    match command {
        "write" => {
            let service = open(opt(opts, "db")?)?;
            let prefix = opt(opts, "prefix")?.to_string();
            let count: usize = opt(opts, "count")?
                .parse()
                .map_err(|err| format!("bad --count: {err}"))?;

            for index in 0..count {
                service
                    .ingest(IngestContextRequest {
                        project: prefix.clone(),
                        ide: "driver".to_string(),
                        file_path: None,
                        language: Some("rust".to_string()),
                        summary: format!("{prefix} record {index}"),
                        body: format!("{prefix} unique body {index} alpha beta gamma"),
                        tags: vec!["driver".to_string()],
                        kind: ContextKind::CodeSnippet,
                        scope: MemoryScope::Personal,
                    })
                    .map_err(|err| err.to_string())?;
            }
            println!("wrote {count}");
        }
        "count" => {
            let store = SqliteVectorStore::open(opt(opts, "db")?).map_err(|err| err.to_string())?;
            println!("{}", store.count().map_err(|err| err.to_string())?);
        }
        "verify" => {
            let store = SqliteVectorStore::open(opt(opts, "db")?).map_err(|err| err.to_string())?;
            println!("sqlite_version={}", store.sqlite_version_number());
            println!(
                "journal_mode={}",
                store.journal_mode().map_err(|err| err.to_string())?
            );
            println!(
                "synchronous={}",
                store.synchronous().map_err(|err| err.to_string())?
            );
            println!(
                "schema_version={}",
                store.schema_version().map_err(|err| err.to_string())?
            );
            println!(
                "busy_timeout_ms={}",
                store.busy_timeout_ms().map_err(|err| err.to_string())?
            );
            println!(
                "integrity={}",
                store.integrity_check().map_err(|err| err.to_string())?
            );
            println!("count={}", store.count().map_err(|err| err.to_string())?);
        }
        "import" => {
            let service = open(opt(opts, "db")?)?;
            let id = opt(opts, "uuid")?
                .parse()
                .map_err(|err| format!("bad --uuid: {err}"))?;
            let scope = match opt(opts, "scope")? {
                "team" => MemoryScope::Team,
                _ => MemoryScope::Personal,
            };

            let record = ContextRecord {
                id,
                project: opt(opts, "project")?.to_string(),
                ide: "driver".to_string(),
                file_path: None,
                language: None,
                summary: format!("imported {id}"),
                body: opt(opts, "body")?.to_string(),
                tags: vec!["import".to_string()],
                kind: ContextKind::Discussion,
                embedding: ContextEmbedding::new(SIMPLE_SHA256_MODEL, Vec::new()),
                created_at: chrono::Utc::now(),
                scope,
                author: Some("driver".to_string()),
                provenance: Some("driver".to_string()),
            };

            let inserted = service
                .import_record(&record)
                .map_err(|err| err.to_string())?;
            println!("inserted={inserted}");
        }
        "embed" => {
            let model = opts
                .get("model")
                .map(String::as_str)
                .unwrap_or(SIMPLE_SHA256_MODEL);
            let engine = Sha256EmbedEngine::try_new(model, 256).map_err(|err| err.to_string())?;
            let vector = engine
                .embed(model, opt(opts, "text")?)
                .map_err(|err| err.to_string())?;
            let hex: String = vector
                .iter()
                .map(|v| format!("{:08x}", v.to_bits()))
                .collect();
            println!("{hex}");
        }
        other => return Err(format!("unknown command `{other}`\n{}", usage())),
    }

    Ok(())
}
