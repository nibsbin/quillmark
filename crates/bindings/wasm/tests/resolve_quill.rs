use quillmark_wasm::{Quill, Quillmark};
use serde_json::Value;
use wasm_bindgen_test::*;

mod common;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_resolve_quill_version() {
    let mut engine = Quillmark::new();

    // Register 0.1.0
    let q1 = Quill::from_tree(common::tree(&[
        (
            "Quill.yaml",
            b"Quill:\n  name: usaf_memo\n  version: \"0.1.0\"\n  backend: typst\n  plate_file: plate.typ\n  description: Version 0.1.0\n",
        ),
        ("plate.typ", b"hello 1"),
    ]))
    .unwrap();
    engine.register_quill(&q1).unwrap();

    // Register 0.2.0
    let q2 = Quill::from_tree(common::tree(&[
        (
            "Quill.yaml",
            b"Quill:\n  name: usaf_memo\n  version: \"0.2.0\"\n  backend: typst\n  plate_file: plate.typ\n  description: Version 0.2.0\n",
        ),
        ("plate.typ", b"hello 2"),
    ]))
    .unwrap();
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
