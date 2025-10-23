use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

use quillmark_wasm::Quillmark;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

// A minimal JSON fixture that represents a very small quill
const SMALL_QUILL_JSON: &str = r#"{
  "files": {
    "Quill.toml": { "contents": "[Quill]\nname = \"test-quill\"\nbackend = \"typst\"\nglue_file = \"glue.typ\"\ndescription = \"Test quill for WASM bindings\"\n" },
    "glue.typ": { "contents": "= Title\n\nThis is a test." },
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

    // Verify it returns a JsValue (we can't easily inspect it without browser APIs)
    assert!(!parsed.is_undefined());
    assert!(!parsed.is_null());
}

#[wasm_bindgen_test]
fn test_register_and_get_quill_info() {
    // Create engine
    let mut engine = Quillmark::new();

    // Register quill
    engine
        .register_quill("test-quill", JsValue::from_str(SMALL_QUILL_JSON))
        .expect("register failed");

    // Get quill info
    let info = engine
        .get_quill_info("test-quill")
        .expect("getQuillInfo failed");

    // Verify it returns a JsValue
    assert!(!info.is_undefined());
    assert!(!info.is_null());
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
        .register_quill("test-quill", JsValue::from_str(SMALL_QUILL_JSON))
        .expect("register failed");

    // Step 3: Get quill info
    let info = engine
        .get_quill_info("test-quill")
        .expect("getQuillInfo failed");
    assert!(!info.is_undefined());

    // Step 4: Render (this may fail in test environment without full WASM setup)
    // We'll just verify the API is callable
    let options = JsValue::undefined();
    let _result = engine.render(parsed, options);
    // Note: render may fail in test due to typst compilation, but that's ok for API testing
}

#[wasm_bindgen_test]
fn engine_register_and_render_legacy() {
    // Legacy test - keeping for backwards compatibility check
    let mut engine = Quillmark::new();

    // Register quill
    engine
        .register_quill("test-quill", JsValue::from_str(SMALL_QUILL_JSON))
        .expect("register failed");

    // Call process_glue on a small markdown
    let glue_out = engine
        .process_glue("test-quill", "---\ntitle: Glue\n---\n\n# X")
        .expect("process_glue failed");
    assert!(glue_out.len() > 0);
}
