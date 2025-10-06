use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

use quillmark_wasm::{Quill, QuillmarkEngine};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

// A minimal JSON fixture that represents a very small quill
const SMALL_QUILL_JSON: &str = r#"{
  "name": "test-quill",
  "Quill.toml": { "contents": "[Quill]\nname = \"test-quill\"\nbackend = \"typst\"\nglue = \"glue.typ\"\n" },
  "glue.typ": { "contents": "= Title\n\nThis is a test." },
  "content.md": { "contents": "---\ntitle: Test\n---\n\n# Hello" }
}"#;

#[wasm_bindgen_test]
fn quill_from_json_and_list_files() {
    // Create a Quill from JSON
    let quill = Quill::from_json(SMALL_QUILL_JSON).expect("from_json failed");

    // Ensure list_files returns expected entries (order not important)
    let files = quill.list_files();
    assert!(files.iter().any(|f| f == "glue.typ"));
    assert!(files.iter().any(|f| f == "Quill.toml"));
}

#[wasm_bindgen_test]
fn engine_and_process_glue() {
    // Create engine
    let mut engine = QuillmarkEngine::create(JsValue::NULL).expect("engine create failed");

    // Register quill
    let quill = Quill::from_json(SMALL_QUILL_JSON).expect("from_json failed");
    engine.register_quill(quill).expect("register failed");

    // Load workflow by name
    let workflow = engine
        .load_workflow("test-quill")
        .expect("load_workflow failed");

    // Call process_glue on a small markdown
    let glue_out = workflow
        .process_glue("---\ntitle: Glue\n---\n\n# X")
        .expect("process_glue failed");
    assert!(glue_out.len() > 0);
}
