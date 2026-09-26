//! Real OS process tests for the concurrent SQLite store.
//!
//! Each test spawns the `ingat_test_driver` binary, so the writes/imports
//! genuinely come from independent processes rather than threads.

#![cfg(feature = "sqlite-store")]

use std::process::{Command, Stdio};

fn driver() -> &'static str {
    env!("CARGO_BIN_EXE_ingat_test_driver")
}

fn run(args: &[&str]) -> String {
    let output = Command::new(driver())
        .args(args)
        .output()
        .expect("spawn ingat_test_driver");
    assert!(
        output.status.success(),
        "driver {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8 stdout")
}

fn spawn(args: &[&str]) -> std::process::Child {
    Command::new(driver())
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn ingat_test_driver")
}

#[test]
fn two_processes_write_200_records() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("memory.sqlite");
    let db = db.to_str().unwrap();

    let mut first = spawn(&["write", "--db", db, "--prefix", "p1", "--count", "100"]);
    let mut second = spawn(&["write", "--db", db, "--prefix", "p2", "--count", "100"]);

    assert!(first.wait().unwrap().success(), "first writer failed");
    assert!(second.wait().unwrap().success(), "second writer failed");

    assert_eq!(run(&["count", "--db", db]).trim(), "200");

    let verify = run(&["verify", "--db", db]);
    assert!(verify.contains("integrity=ok"), "{verify}");
    assert!(verify.contains("journal_mode=wal"), "{verify}");
    assert!(verify.contains("synchronous=2"), "{verify}");
    assert!(verify.contains("schema_version=1"), "{verify}");
}

#[test]
fn two_processes_import_same_uuid_insert_once() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("memory.sqlite");
    let db = db.to_str().unwrap();

    // Initialise the schema first so both importers race on the insert only.
    run(&["count", "--db", db]);

    let uuid = "018f2b7c-0000-7000-8000-000000000001";
    let args = [
        "import",
        "--db",
        db,
        "--uuid",
        uuid,
        "--project",
        "kode",
        "--body",
        "shared body",
        "--scope",
        "team",
    ];

    let mut first = spawn(&args);
    let mut second = spawn(&args);

    let first_out = first.wait_with_output().unwrap();
    let second_out = second.wait_with_output().unwrap();
    assert!(first_out.status.success());
    assert!(second_out.status.success());

    let stdout_first = String::from_utf8(first_out.stdout).unwrap();
    let stdout_second = String::from_utf8(second_out.stdout).unwrap();
    let total_inserted = [stdout_first.as_str(), stdout_second.as_str()]
        .iter()
        .filter(|out| out.contains("inserted=true"))
        .count();

    assert_eq!(
        total_inserted, 1,
        "exactly one process must report the insertion (first={stdout_first:?}, second={stdout_second:?})"
    );
    assert_eq!(run(&["count", "--db", db]).trim(), "1");
}
