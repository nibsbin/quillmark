use crate::document::tests::collect_md_files;

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

/// Emit, the wire and the storage DTO all read a root `!must_fill` off the
/// payload item's flag, so a value tree whose own root bit disagrees with that
/// flag compares as a different `Document` than it emits. A caller's root bit
/// is normalized on the way in, and both round-trips return an equal document
/// whether the field is marked or not.
#[test]
fn a_root_fill_bit_on_a_stored_value_round_trips() {
    use crate::document::Document;
    use crate::value::QuillValue;

    let mut marked = QuillValue::from_json(serde_json::json!("draft"));
    assert!(marked.set_fill_at(&[]));

    let mut doc = Document::new("q@1.0.0".parse().expect("reference"));
    doc.main_mut()
        .store_fields([("x".to_string(), marked.clone())])
        .expect("store_fields accepts the value");
    doc.main_mut()
        .store_field("y", marked.clone())
        .expect("store_field accepts the value");
    doc.main_mut()
        .store_fill("z", marked)
        .expect("store_fill accepts the value");

    let md = doc.to_markdown();
    assert!(md.contains("\nx: draft\n"), "{md}");
    assert!(md.contains("\ny: draft\n"), "{md}");
    assert!(md.contains("\nz: !must_fill draft\n"), "{md}");

    let reparsed = Document::parse(&md)
        .expect("the emitted document re-parses")
        .document;
    assert_eq!(reparsed, doc, "markdown round-trip:\n{md}");

    let json = serde_json::to_string(&doc).expect("to_json");
    let restored: Document = serde_json::from_str(&json).expect("from_json");
    assert_eq!(restored, doc, "storage DTO round-trip");
}
