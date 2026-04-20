use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

use quillmark_wasm::{ParsedDocument, Quillmark, RenderOptions};

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

const SIMPLE_MARKDOWN: &str = "---\nQUILL: test_quill\ntitle: Hello\n---\n\n# Hello\n";

#[wasm_bindgen_test]
fn test_parse_markdown_static() {
    let parsed = ParsedDocument::from_markdown(SIMPLE_MARKDOWN).expect("fromMarkdown failed");
    assert_eq!(parsed.quill_ref, "test_quill");
}

#[wasm_bindgen_test]
fn test_quill_from_tree() {
    let engine = Quillmark::new();
    let quill = engine.quill(small_quill_tree()).expect("quill failed");
    let _ = quill;
}

#[wasm_bindgen_test]
/// Rendering markdown with a QUILL ref that differs from the quill name must yield
/// exactly one warning with code `quill::ref_mismatch` and still produce an artifact.
#[wasm_bindgen_test]
fn test_render_ref_mismatch_warning() {
    let engine = Quillmark::new();
    let quill = engine.quill(small_quill_tree()).expect("quill failed");

    // Document declares a different quill name than the loaded quill ("test_quill")
    let mismatch_md = "---\nQUILL: other_quill\ntitle: Mismatch\n---\n\n# Content\n";
    let result = quill
        .render(JsValue::from_str(mismatch_md), RenderOptions::default())
        .expect("render should succeed despite mismatch");

    assert_eq!(result.warnings.len(), 1, "expected exactly one warning");
    assert_eq!(
        result.warnings[0].code.as_deref(),
        Some("quill::ref_mismatch"),
        "warning code should be quill::ref_mismatch"
    );
    assert!(!result.artifacts.is_empty(), "artifact must be produced");
}

/// `quill.render(markdown_string, opts)` — render via raw Markdown string input.
#[wasm_bindgen_test]
fn test_render_from_string() {
    let engine = Quillmark::new();
    let quill = engine.quill(small_quill_tree()).expect("quill failed");

    let result = quill
        .render(JsValue::from_str(SIMPLE_MARKDOWN), RenderOptions::default())
        .expect("render from string failed");

    assert!(
        !result.artifacts.is_empty(),
        "should produce at least one artifact"
    );
    assert_eq!(
        result.warnings.len(),
        0,
        "no warnings expected for matching quill_ref"
    );
}

/// `quill.render(ParsedDocument, opts)` — render via pre-parsed document.
#[wasm_bindgen_test]
fn test_render_from_parsed_document() {
    let engine = Quillmark::new();
    let quill = engine.quill(small_quill_tree()).expect("quill failed");

    let parsed = ParsedDocument::from_markdown(SIMPLE_MARKDOWN).expect("fromMarkdown failed");
    // Convert to JsValue so the engine's input-type dispatch treats it as ParsedDocument
    let parsed_js =
        serde_wasm_bindgen::to_value(&parsed).expect("ParsedDocument serialization failed");

    let result = quill
        .render(parsed_js, RenderOptions::default())
        .expect("render from ParsedDocument failed");

    assert!(
        !result.artifacts.is_empty(),
        "should produce at least one artifact"
    );
}
