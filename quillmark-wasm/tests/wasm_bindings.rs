use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

use quillmark_wasm::Quillmark;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

// A minimal JSON fixture that represents a very small quill
const SMALL_QUILL_JSON: &str = r#"{
  "Quill.toml": { "contents": "[Quill]\nname = \"test-quill\"\nbackend = \"typst\"\nglue = \"glue.typ\"\n" },
  "glue.typ": { "contents": "= Title\n\nThis is a test." },
  "content.md": { "contents": "---\ntitle: Test\n---\n\n# Hello" }
}"#;

#[wasm_bindgen_test]
fn engine_register_and_render() {
    // Create engine
    let mut engine = Quillmark::create();

    // Register quill
    engine
        .register_quill("test-quill", JsValue::from_str(SMALL_QUILL_JSON))
        .expect("register failed");

    // Call render_glue on a small markdown
    let glue_out = engine
        .render_glue("test-quill", "---\ntitle: Glue\n---\n\n# X")
        .expect("render_glue failed");
    assert!(glue_out.len() > 0);
}
