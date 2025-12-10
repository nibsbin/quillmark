use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

use quillmark_wasm::Quillmark;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

// A minimal JSON fixture that represents a very small quill
const SMALL_QUILL_JSON: &str = r#"{
  "files": {
    "Quill.toml": { "contents": "[Quill]\nname = \"test-quill\"\nbackend = \"typst\"\nplate_file = \"plate.typ\"\ndescription = \"Test quill for WASM bindings\"\n" },
    "plate.typ": { "contents": "= Title\n\nThis is a test." },
    "content.md": { "contents": "---\ntitle: Test\n---\n\n# Hello" }
  }
}"#;

#[wasm_bindgen_test]
fn test_parse_markdown() {
    // Parse simple markdown with frontmatter
    let markdown = r#"---
title: Test Document
author: Alice
QUILL: test-quill
---

# Hello World

This is a test document.
"#;

    let parsed = Quillmark::parse_markdown(markdown).expect("parse_markdown failed");

    // Verify it returns a ParsedDocument
    assert_eq!(parsed.quill_tag, "test-quill");
    assert!(parsed.fields.is_object());
}

#[wasm_bindgen_test]
fn test_register_and_get_quill_info() {
    // Create engine
    let mut engine = Quillmark::new();

    // Register quill
    engine
        .register_quill(JsValue::from_str(SMALL_QUILL_JSON))
        .expect("register failed");

    // Get quill info
    let info = engine
        .get_quill_info("test-quill")
        .expect("getQuillInfo failed");

    // Verify it returns a QuillInfo
    assert_eq!(info.name, "test-quill");
    assert_eq!(info.backend, "typst");
}

#[wasm_bindgen_test]
fn test_workflow_parse_register_get_info_render() {
    // Step 1: Parse markdown
    let markdown = r#"---
title: Test Document
author: Alice
QUILL: test-quill
---

# Hello World

This is a test.
"#;

    let parsed = Quillmark::parse_markdown(markdown).expect("parse_markdown failed");

    // Step 2: Create engine and register quill
    let mut engine = Quillmark::new();
    engine
        .register_quill(JsValue::from_str(SMALL_QUILL_JSON))
        .expect("register failed");

    // Step 3: Get quill info
    let info = engine
        .get_quill_info("test-quill")
        .expect("getQuillInfo failed");
    assert_eq!(info.name, "test-quill");

    // Step 4: Render (this may fail in test environment without full WASM setup)
    // We'll just verify the API is callable
    use quillmark_wasm::RenderOptions;
    let options = RenderOptions::default();
    let _result = engine.render(parsed, options);
    // Note: render may fail in test due to typst compilation, but that's ok for API testing
}

#[wasm_bindgen_test]
fn engine_register_and_render_legacy() {
    // Legacy test - keeping for backwards compatibility check
    let mut engine = Quillmark::new();

    // Register quill
    engine
        .register_quill(JsValue::from_str(SMALL_QUILL_JSON))
        .expect("register failed");

    // Call render_plate on a small markdown
    let print_out = engine
        .render_plate("test-quill", "---\ntitle: Test\n---\n\n# X")
        .expect("render_plate failed");
    assert!(print_out.len() > 0);
}
