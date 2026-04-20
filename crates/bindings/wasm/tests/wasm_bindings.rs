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

/// Rendering with a QUILL ref that differs from the quill name must yield
/// exactly one warning with code `quill::ref_mismatch` and still produce an artifact.
#[wasm_bindgen_test]
fn test_render_ref_mismatch_warning() {
    let engine = Quillmark::new();
    let quill = engine.quill(small_quill_tree()).expect("quill failed");

    let mismatch_md = "---\nQUILL: other_quill\ntitle: Mismatch\n---\n\n# Content\n";
    let parsed = ParsedDocument::from_markdown(mismatch_md).expect("fromMarkdown failed");
    let result = quill
        .render(parsed, RenderOptions::default())
        .expect("render should succeed despite mismatch");

    assert_eq!(result.warnings.len(), 1, "expected exactly one warning");
    assert_eq!(
        result.warnings[0].code.as_deref(),
        Some("quill::ref_mismatch"),
        "warning code should be quill::ref_mismatch"
    );
    assert!(!result.artifacts.is_empty(), "artifact must be produced");
}

/// `quill.render(ParsedDocument, opts)` — render via pre-parsed document.
#[wasm_bindgen_test]
fn test_render_from_parsed_document() {
    let engine = Quillmark::new();
    let quill = engine.quill(small_quill_tree()).expect("quill failed");

    let parsed = ParsedDocument::from_markdown(SIMPLE_MARKDOWN).expect("fromMarkdown failed");
    let result = quill
        .render(parsed, RenderOptions::default())
        .expect("render from ParsedDocument failed");

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

/// `quill.open(ParsedDocument)` returns a render session supporting page_count + render.
#[wasm_bindgen_test]
fn test_open_session_render() {
    let engine = Quillmark::new();
    let quill = engine.quill(small_quill_tree()).expect("quill failed");

    let parsed = ParsedDocument::from_markdown(SIMPLE_MARKDOWN).expect("fromMarkdown failed");
    let session = quill.open(parsed).expect("open failed");
    assert!(session.page_count() > 0, "session should expose page count");

    let result = session
        .render(RenderOptions::default())
        .expect("session render failed");
    assert!(!result.artifacts.is_empty(), "should produce artifacts");
}
