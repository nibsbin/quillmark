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
    ])
}

#[wasm_bindgen_test]
fn test_parse_markdown() {
    let markdown = r#"---
title: Test Document
author: Alice
QUILL: test_quill
---

# Hello World
"#;
    let parsed = Quillmark::parse_markdown(markdown).expect("parse_markdown failed");
    assert_eq!(parsed.quill_ref, "test_quill");
    assert!(parsed.fields.is_object());
}

#[wasm_bindgen_test]
fn test_parse_markdown_static() {
    use quillmark_wasm::ParsedDocument;
    let markdown = "---\nQUILL: test_quill\ntitle: Hello\n---\n\n# Hello\n";
    let parsed = ParsedDocument::from_markdown(markdown).expect("fromMarkdown failed");
    assert_eq!(parsed.quill_ref, "test_quill");
}

#[wasm_bindgen_test]
fn test_quill_from_tree() {
    let engine = Quillmark::new();
    let quill = engine.quill_from_tree(small_quill_tree()).expect("quillFromTree failed");
    // Quill should be render-ready after quillFromTree
    let _ = quill;
}

#[wasm_bindgen_test]
fn test_quill_from_tree_static() {
    let quill = Quill::from_tree(small_quill_tree()).expect("fromTree failed");
    let _ = quill;
}
