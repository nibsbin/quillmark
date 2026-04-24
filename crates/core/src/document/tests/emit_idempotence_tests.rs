//! Emit-idempotence corpus tests
//!
//! `doc.to_markdown()` must be a pure function of `doc`: two calls return
//! byte-equal strings.  These tests run that invariant over the full fixture
//! corpus.
//!

use crate::document::Document;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Collect all `.md` files reachable from `root`, walking recursively.
fn collect_md_files(root: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let entries = match std::fs::read_dir(root) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_md_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

// ── Corpus idempotence ────────────────────────────────────────────────────────

/// For every parseable `.md` in the fixture corpus: `to_markdown()` called
/// twice on the same `Document` must return byte-equal strings.
#[test]
fn emit_idempotence_over_fixture_corpus() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let resources_dir = std::path::Path::new(manifest_dir)
        .join("..")
        .join("fixtures")
        .join("resources");

    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    collect_md_files(&resources_dir, &mut paths);

    assert!(
        !paths.is_empty(),
        "no fixture files found under {}",
        resources_dir.display()
    );

    let mut passed = 0usize;
    let mut skipped = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for path in &paths {
        let label = path.to_string_lossy();
        let src = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("SKIP {}: read error: {}", label, e);
                skipped += 1;
                continue;
            }
        };

        let doc = match Document::from_markdown(&src) {
            Ok(d) => d,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };

        let first = doc.to_markdown();
        let second = doc.to_markdown();

        if first == second {
            passed += 1;
        } else {
            failures.push(format!(
                "FAIL {}: to_markdown() not idempotent\nFirst  (first 400 chars): {:.400}\nSecond (first 400 chars): {:.400}",
                label, first, second
            ));
        }
    }

    if !failures.is_empty() {
        panic!(
            "Emit-idempotence failures ({} failed, {} passed, {} skipped):\n{}",
            failures.len(),
            passed,
            skipped,
            failures.join("\n\n")
        );
    }

    assert!(
        passed > 0,
        "No fixtures passed idempotence check — did all files get skipped?"
    );

    eprintln!(
        "emit_idempotence_over_fixture_corpus: {} passed, {} skipped",
        passed, skipped
    );
}
