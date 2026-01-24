use quillmark_wasm::Quillmark;
use serde_json::Value;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

// wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

const UI_QUILL_JSON: &str = r#"{
  "files": {
    "Quill.yaml": { "contents": "Quill:\n  name: ui-test-quill\n  backend: typst\n  plate_file: plate.typ\n  description: Test quill for UI metadata\n\nfields:\n  my_field:\n    type: string\n    ui:\n      group: Personal Info\n" },
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
        .get_quill_info("ui-test-quill")
        .expect("getQuillInfo failed");

    // Navigate to schema.properties.my_field.x-ui
    let x_ui = info
        .schema
        .pointer("/properties/my_field/x-ui")
        .expect("x-ui not found");

    assert_eq!(x_ui["group"], "Personal Info");
    assert_eq!(x_ui["order"], 0);
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

    // Get full info
    let info = engine
        .get_quill_info("ui-test-quill")
        .expect("getQuillInfo failed");

    // Get stripped schema using the helper method
    let stripped_schema = info.get_stripped_schema();

    // Verify x-ui is GONE in stripped schema
    let x_ui = stripped_schema.pointer("/properties/my_field/x-ui");
    assert!(x_ui.is_none(), "x-ui should be stripped");

    // Verify other fields remain in stripped schema
    let field_type = stripped_schema
        .pointer("/properties/my_field/type")
        .expect("type should exist");
    assert_eq!(field_type, "string");

    // Verify original info still has x-ui
    let x_ui_original = info.schema.pointer("/properties/my_field/x-ui");
    assert!(
        x_ui_original.is_some(),
        "original schema should still have x-ui"
    );
}
