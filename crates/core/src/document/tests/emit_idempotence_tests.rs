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

/// A comment between a bare `-` and the item's first key belongs to the item, so
/// it re-emits inside the item and the first emit is already the fixed point.
#[test]
fn a_comment_before_a_sequence_item_first_key_stays_inside_the_item() {
    use crate::document::Document;

    let src = "\
~~~
$quill: test@1.0
$kind: main
items:
  -
    # c
    name: a
~~~

Body.
";
    let doc = Document::parse(src).expect("parses").document;
    let md = doc.to_markdown();
    assert_eq!(md, src, "the first emit is not the fixed point");

    let reparsed = Document::parse(&md)
        .expect("the emitted document re-parses")
        .document;
    assert_eq!(doc, reparsed, "emit is not a fixed point: {md}");
}
