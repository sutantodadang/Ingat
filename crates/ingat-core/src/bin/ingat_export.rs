//! `ingat_export` — offline migration of a stopped sled store to JSONL.
//!
//! ```text
//! ingat_export --store <stopped-sled-store> --out <jsonl>
//! ```
//!
//! Fails if the store is active/locked. Writes a `{"v":1,"record":...}` JSON
//! line per record and prints only counts.

use std::path::PathBuf;
use std::process::ExitCode;

use ingat_core::infrastructure::export_legacy_jsonl;

fn usage() -> String {
    "usage: ingat_export --store <stopped-sled-store> --out <jsonl>".to_string()
}

fn parse_args() -> Result<(PathBuf, PathBuf), String> {
    let mut store: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--store" => store = args.next().map(PathBuf::from),
            "--out" => out = args.next().map(PathBuf::from),
            "-h" | "--help" => return Err(usage()),
            other => return Err(format!("unknown argument `{other}`\n{}", usage())),
        }
    }

    match (store, out) {
        (Some(store), Some(out)) => Ok((store, out)),
        _ => Err(usage()),
    }
}

fn main() -> ExitCode {
    let (store, out) = match parse_args() {
        Ok(paths) => paths,
        Err(message) => {
            eprintln!("ingat_export: {message}");
            return ExitCode::FAILURE;
        }
    };

    match export_legacy_jsonl(&store, &out) {
        Ok(count) => {
            println!("exported {count} record(s) to {}", out.display());
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("ingat_export: {err}");
            ExitCode::FAILURE
        }
    }
}
