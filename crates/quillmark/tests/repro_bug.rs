use quillmark::Quillmark;
use quillmark_core::{value::QuillValue, Quill};

#[test]
fn test_get_quill_bug() {
    let mut engine = Quillmark::new();

    let mut q1 = Quill::from_json(r#"{"name":"usaf_memo","backend":"typst"}"#).unwrap();
    q1.metadata.insert(
        "version".to_string(),
        QuillValue::from_json(serde_json::json!("0.1.0")),
    );
    engine.register_quill(q1).unwrap();

    let mut q2 = Quill::from_json(r#"{"name":"usaf_memo","backend":"typst"}"#).unwrap();
    q2.metadata.insert(
        "version".to_string(),
        QuillValue::from_json(serde_json::json!("0.2.0")),
    );
    engine.register_quill(q2).unwrap();

    let resolved = engine.get_quill("usaf_memo@0.2.0").unwrap();
    assert_eq!(
        resolved.metadata.get("version").unwrap().as_str().unwrap(),
        "0.2.0"
    );
}
