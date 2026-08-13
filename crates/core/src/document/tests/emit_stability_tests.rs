use crate::document::Document;
use crate::document::tests::collect_md_files;

#[test]
fn parse_emit_parse_emit_stability_over_fixtures() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let resources_dir = std::path::Path::new(manifest_dir)
        .join("..")
        .join("fixtures")
        .join("resources");

    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    collect_md_files(&resources_dir, &mut paths);

    assert!(!paths.is_empty(), "no fixture files found");

    let mut passed = 0usize;
    let mut skipped = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for path in &paths {
        let label = path.to_string_lossy();
        let src = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };

        let a = match Document::parse(&src) {
            Ok(d) => d.document,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };

        let emit1 = a.to_markdown();

        let b = match Document::parse(&emit1) {
            Ok(d) => d.document,
            Err(e) => {
                failures.push(format!(
                    "FAIL {}: re-parse of emit1 failed: {}\nEmit1:\n{}",
                    label, e, emit1
                ));
                continue;
            }
        };

        let emit2 = b.to_markdown();

        if emit1 == emit2 {
            passed += 1;
        } else {
            failures.push(format!(
                "FAIL {}: emit1 != emit2\nEmit1:\n{}\nEmit2:\n{}",
                label, emit1, emit2
            ));
        }
    }

    if !failures.is_empty() {
        panic!(
            "Parse∘Emit∘Parse∘Emit stability failures ({} failed, {} passed, {} skipped):\n{}",
            failures.len(),
            passed,
            skipped,
            failures.join("\n\n")
        );
    }

    assert!(
        passed > 0,
        "No fixtures passed stability check: did all files get skipped?"
    );

    eprintln!(
        "parse_emit_parse_emit_stability: {} passed, {} skipped",
        passed, skipped
    );
}
