//! Stable `ingat/simple-sha256-v1` embeddings across independently started
//! processes (and therefore across restarts).

#![cfg(feature = "sqlite-store")]

use std::process::Command;

fn driver() -> &'static str {
    env!("CARGO_BIN_EXE_ingat_test_driver")
}

fn embed_hex(text: &str) -> String {
    let output = Command::new(driver())
        .args(["embed", "--text", text])
        .output()
        .expect("spawn ingat_test_driver embed");
    assert!(
        output.status.success(),
        "embed failed for {text:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

#[test]
fn embeddings_are_identical_across_processes_and_restart() {
    let fixtures = [
        "hello world",
        "Halo dunia, ini teks Bahasa Indonesia",
        "日本語のテキストと漢字",
        "emoji 🚀 and spacing\tnewlines\nhere",
    ];

    for text in fixtures {
        let first = embed_hex(text);
        let second = embed_hex(text);
        assert!(!first.is_empty(), "empty embedding for {text:?}");
        assert_eq!(
            first, second,
            "embedding for {text:?} must be process-stable"
        );
    }
}
