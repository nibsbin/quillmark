use crate::document::tests::{collect_md_files, parse};

#[test]
fn markdown_and_json_converge_on_canonical_form() {
    use crate::document::Document;

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has parent")
        .parent()
        .expect("crates dir has parent");
    let fixtures_root = workspace_root.join("crates/fixtures/resources");

    let mut all_md = Vec::new();
    collect_md_files(&fixtures_root, &mut all_md);

    let mut passed = 0;
    let mut skipped = 0;
    let mut failures = Vec::new();

    for path in &all_md {
        let label = path
            .strip_prefix(workspace_root)
            .unwrap_or(path)
            .display()
            .to_string();

        let Ok(src) = std::fs::read_to_string(path) else {
            skipped += 1;
            continue;
        };
        let Ok(doc) = Document::parse(&src).map(|p| p.document) else {
            skipped += 1;
            continue;
        };

        let md_canonical = doc.to_markdown();

        let json = serde_json::to_string(&doc).expect("to_json should succeed");
        let restored: Document = serde_json::from_str(&json).expect("from_json should round-trip");
        let md_after_json_round = restored.to_markdown();

        if md_canonical == md_after_json_round {
            passed += 1;
        } else {
            failures.push(format!(
                "FAIL {}: markdown/JSON canonical forms diverge\nMarkdown direct:    {:.400}\nThrough JSON DTO:   {:.400}",
                label, md_canonical, md_after_json_round
            ));
        }
    }

    if !failures.is_empty() {
        panic!(
            "Canonical-convergence failures ({} failed, {} passed, {} skipped):\n{}",
            failures.len(),
            passed,
            skipped,
            failures.join("\n\n")
        );
    }

    assert!(passed > 0, "no fixtures passed convergence check");
    eprintln!(
        "markdown_and_json_converge_on_canonical_form: {} passed, {} skipped",
        passed, skipped
    );
}

#[test]
fn synthesised_kind_leaves_the_quill_trailer_on_quill() {
    let src = "~~~card-yaml\n$quill: q@1.0 # note on quill\ntitle: x\n~~~\n";
    let doc = parse(src);

    let emitted = doc.to_markdown();
    assert!(
        emitted.contains("$quill: q@1.0 # note on quill\n$kind: main\n"),
        "trailer belongs to $quill, not to the synthesised $kind\nGot:\n{}",
        emitted
    );
    assert_eq!(
        parse(&emitted),
        doc,
        "emit must re-parse to the same document"
    );
}

#[test]
fn store_ext_leaves_the_kind_trailer_on_kind() {
    let src = "~~~card-yaml\n$quill: q@1.0\n$kind: main # note on kind\ntitle: x\n~~~\n";
    let mut doc = parse(src);

    let mut ext = serde_json::Map::new();
    ext.insert("editor".into(), serde_json::json!({ "pinned": true }));
    doc.main_mut().store_ext(ext).expect("shallow map stores");

    let emitted = doc.to_markdown();
    assert!(
        emitted.contains("$kind: main # note on kind\n$ext:\n"),
        "trailer belongs to $kind, not to the new $ext\nGot:\n{}",
        emitted
    );
    assert_eq!(
        parse(&emitted),
        doc,
        "emit must re-parse to the same document"
    );
}
