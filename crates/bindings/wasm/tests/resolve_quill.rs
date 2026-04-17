use quillmark_wasm::{Quill, Quillmark};
use serde_json::Value;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_resolve_quill_version() {
    let mut engine = Quillmark::new();

    // Register 0.1.0
    let q1 = serde_json::json!({
      "files": {
        "Quill.yaml": { "contents": "Quill:\n  name: usaf_memo\n  version: \"0.1.0\"\n  backend: typst\n  plate_file: plate.typ\n  description: Version 0.1.0\n" },
        "plate.typ": { "contents": "hello 1" }
      }
    });
    let q1 = Quill::from_json(wasm_bindgen::JsValue::from_str(&q1.to_string())).unwrap();
    engine.register_quill(&q1).unwrap();

    // Register 0.2.0
    let q2 = serde_json::json!({
      "files": {
        "Quill.yaml": { "contents": "Quill:\n  name: usaf_memo\n  version: \"0.2.0\"\n  backend: typst\n  plate_file: plate.typ\n  description: Version 0.2.0\n" },
        "plate.typ": { "contents": "hello 2" }
      }
    });
    let q2 = Quill::from_json(wasm_bindgen::JsValue::from_str(&q2.to_string())).unwrap();
    engine.register_quill(&q2).unwrap();

    // Resolve 0.2.0
    let js_val = engine.resolve_quill("usaf_memo@0.2.0");
    let info: Value = serde_wasm_bindgen::from_value(js_val).expect("resolveQuill json");
    assert_eq!(info.get("name").and_then(Value::as_str), Some("usaf_memo"));
    assert_eq!(
        info.get("metadata")
            .and_then(|m| m.get("version"))
            .and_then(Value::as_str),
        Some("0.2.0")
    );
}
