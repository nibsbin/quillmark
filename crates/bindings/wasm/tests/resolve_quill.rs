use quillmark_wasm::Quillmark;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_resolve_quill_version() {
    let mut engine = Quillmark::new();

    // Register 0.1.0
    let q1 = serde_json::json!({
        "name": "usaf_memo",
        "backend": "typst",
        "metadata": { "version": "0.1.0" },
        "schema": {},
        "plate": "hello 1"
    });
    engine
        .register_quill(wasm_bindgen::JsValue::from_str(&q1.to_string()))
        .unwrap();

    // Register 0.2.0
    let q2 = serde_json::json!({
        "name": "usaf_memo",
        "backend": "typst",
        "metadata": { "version": "0.2.0" },
        "schema": {},
        "plate": "hello 2"
    });
    engine
        .register_quill(wasm_bindgen::JsValue::from_str(&q2.to_string()))
        .unwrap();

    // Resolve 0.2.0
    let js_val = engine.resolve_quill("usaf_memo@0.2.0");
    // Verify it picked 0.2.0
    // But how to parse JsValue back?
}
