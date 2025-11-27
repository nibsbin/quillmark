use quillmark_wasm::Quillmark;
use serde_json::Value;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

// wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

const UI_QUILL_JSON: &str = r#"{
  "files": {
    "Quill.toml": { "contents": "[Quill]\nname = \"ui-test-quill\"\nbackend = \"typst\"\nglue_file = \"glue.typ\"\ndescription = \"Test quill for UI metadata\"\n\n[fields.my_field]\ntype = \"string\"\n\n[fields.my_field.ui]\ngroup = \"Personal Info\"\ntooltip = \"Enter your name\"\nextra = { placeholder = \"John Doe\" }\n" },
    "glue.typ": { "contents": "= Title" }
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

    let info_js = engine
        .get_quill_info("ui-test-quill")
        .expect("getQuillInfo failed");

    // Convert JsValue to serde_json::Value to inspect it easily in Rust
    let info: Value = serde_wasm_bindgen::from_value(info_js).expect("failed to deserialize info");

    // Navigate to schema.properties.my_field.x-ui
    let x_ui = info
        .pointer("/schema/properties/my_field/x-ui")
        .expect("x-ui not found");

    assert_eq!(x_ui["group"], "Personal Info");
    assert_eq!(x_ui["tooltip"], "Enter your name");
    assert_eq!(x_ui["extra"]["placeholder"], "John Doe");
}
