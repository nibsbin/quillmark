use quillmark_wasm::Quillmark;
use serde_json::Value;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

// wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

const UI_QUILL_JSON: &str = r#"{
  "files": {
    "Quill.yaml": { "contents": "Quill:\n  name: ui_test_quill\n  version: \"0.1\"\n  backend: typst\n  plate_file: plate.typ\n  description: Test quill for UI metadata\n\nmain:\n  fields:\n    my_field:\n      type: string\n      ui:\n        group: Personal Info\n" },
    "plate.typ": { "contents": "= Title" }
  }
}"#;

#[wasm_bindgen_test]
fn test_metadata_retrieval() {
    let mut engine = Quillmark::new();
    engine
        .register_quill(JsValue::from_str(UI_QUILL_JSON))
        .map_err(|e| {
            let error_obj: Value = serde_wasm_bindgen::from_value(e).unwrap();
            panic!("register failed: {:#?}", error_obj);
        })
        .unwrap();

    let info = engine
        .get_quill_info("ui-test_quill")
        .expect("getQuillInfo failed");

    let schema: serde_yaml::Value = serde_yaml::from_str(&info.schema).expect("schema yaml");
    let ui = schema
        .get("fields")
        .and_then(|v| v.get("my_field"))
        .and_then(|v| v.get("ui"))
        .expect("ui not found");

    assert_eq!(
        ui.get("group").and_then(|v| v.as_str()),
        Some("Personal Info")
    );
    assert_eq!(ui.get("order").and_then(|v| v.as_i64()), Some(0));
}

#[wasm_bindgen_test]
fn test_metadata_stripping() {
    let mut engine = Quillmark::new();
    engine
        .register_quill(JsValue::from_str(UI_QUILL_JSON))
        .map_err(|e| {
            let error_obj: Value = serde_wasm_bindgen::from_value(e).unwrap();
            panic!("register failed: {:#?}", error_obj);
        })
        .unwrap();

    let schema_yaml = engine
        .get_quill_schema("ui-test_quill")
        .expect("getQuillSchema failed");
    let schema: serde_yaml::Value = serde_yaml::from_str(&schema_yaml).expect("schema yaml");

    // Verify native `ui` is present and old JSON-schema-specific keys are absent.
    assert!(schema
        .get("fields")
        .and_then(|v| v.get("my_field"))
        .and_then(|v| v.get("ui"))
        .is_some());
    assert!(schema.get("CARDS").is_none());
    assert!(!schema_yaml.contains("x-"));
}
