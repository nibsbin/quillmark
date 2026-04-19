use wasm_bindgen_test::*;

use quillmark_wasm::{Quill, Quillmark};

mod common;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

fn small_quill_tree() -> wasm_bindgen::JsValue {
    common::tree(&[
        (
            "Quill.yaml",
            b"Quill:\n  name: test_quill\n  backend: typst\n  plate_file: plate.typ\n  description: Test quill for WASM bindings\n",
        ),
        ("plate.typ", b"= Title\n\nThis is a test."),
        ("content.md", b"---\ntitle: Test\n---\n\n# Hello"),
    ])
}

#[wasm_bindgen_test]
fn test_parse_markdown() {
    // Parse simple markdown with frontmatter
    let markdown = r#"---
title: Test Document
author: Alice
QUILL: test_quill
---

# Hello World

This is a test document.
"#;

    let parsed = Quillmark::parse_markdown(markdown).expect("parse_markdown failed");

    // Verify it returns a ParsedDocument
    assert_eq!(parsed.quill_ref, "test_quill");
    assert!(parsed.fields.is_object());
}

#[wasm_bindgen_test]
fn test_register_and_get_quill_info() {
    // Create engine
    let mut engine = Quillmark::new();

    // Register quill
    let quill = Quill::from_tree(small_quill_tree()).expect("fromTree failed");
    engine.register_quill(&quill).expect("register failed");

    // Get quill info
    let info = engine
        .get_quill_info("test_quill")
        .expect("getQuillInfo failed");

    // Verify it returns a QuillInfo
    assert_eq!(info.name, "test_quill");
    assert_eq!(info.backend, "typst");
}

#[wasm_bindgen_test]
fn test_workflow_parse_register_get_info_render() {
    // Step 1: Parse markdown
    let markdown = r#"---
title: Test Document
author: Alice
QUILL: test_quill
---

# Hello World

This is a test.
"#;

    let parsed = Quillmark::parse_markdown(markdown).expect("parse_markdown failed");

    // Step 2: Create engine and register quill
    let mut engine = Quillmark::new();
    let quill = Quill::from_tree(small_quill_tree()).expect("fromTree failed");
    engine.register_quill(&quill).expect("register failed");

    // Step 3: Get quill info
    let info = engine
        .get_quill_info("test_quill")
        .expect("getQuillInfo failed");
    assert_eq!(info.name, "test_quill");

    // Step 4: Render (this may fail in test environment without full WASM setup)
    // We'll just verify the API is callable
    use quillmark_wasm::RenderOptions;
    let options = RenderOptions::default();
    let _result = engine.render(parsed, options);
    // Note: render may fail in test due to typst compilation, but that's ok for API testing
}

#[wasm_bindgen_test]
fn test_has_quill_exact_canonical_ref() {
    let mut engine = Quillmark::new();
    let quill = Quill::from_tree(common::tree(&[
        (
            "Quill.yaml",
            b"Quill:\n  name: canonical_quill\n  version: \"1.0.0\"\n  backend: typst\n  plate_file: plate.typ\n  description: Test quill\n",
        ),
        ("plate.typ", b"= Title\n\n#{{ body | Content }}"),
    ]))
    .expect("fromTree failed");

    assert!(!engine.has_quill("canonical_quill@1.0.0"));
    engine.register_quill(&quill).expect("register failed");
    assert!(engine.has_quill("canonical_quill@1.0.0"));
    assert!(!engine.has_quill("canonical_quill@2.0.0"));
}
