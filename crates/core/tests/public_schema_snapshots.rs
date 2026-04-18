use std::fs;

use quillmark_core::Quill;

#[test]
fn public_schema_snapshot_usaf_memo_0_1_0() {
    let quill_path = quillmark_fixtures::resource_path("quills/usaf_memo/0.1.0");
    let quill = Quill::from_path(quill_path).expect("failed to load usaf_memo fixture");

    let yaml = quill
        .config
        .public_schema_yaml()
        .expect("failed to emit public schema yaml");

    let expected_path =
        quillmark_fixtures::resource_path("quills/usaf_memo/0.1.0/__golden__/public_schema.yaml");
    let expected = fs::read_to_string(expected_path).expect("failed to read golden public schema");

    assert_eq!(yaml, expected, "public schema snapshot changed");

    let parsed: serde_json::Value =
        serde_saphyr::from_str(&yaml).expect("schema yaml should parse");
    assert!(parsed.get("name").is_some());
    assert!(parsed.get("fields").is_some());
    assert!(parsed.get("cards").is_some());
    assert!(parsed.get("CARDS").is_none());
}
