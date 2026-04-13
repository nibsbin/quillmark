use std::collections::HashMap;

use quillmark_core::schema::{
    build_schema_from_config, coerce_document, extract_defaults_from_schema,
    extract_examples_from_schema,
};
use quillmark_core::{ParsedDocument, Quill, QuillValue};

fn fixture_names() -> [&'static str; 1] {
    ["usaf_memo"]
}

fn load_quill(name: &str) -> Quill {
    Quill::from_path(quillmark_fixtures::quills_path(name)).expect("failed to load fixture quill")
}

fn load_example_fields(name: &str) -> HashMap<String, QuillValue> {
    let example_path = quillmark_fixtures::quills_path(name).join("example.md");
    let markdown =
        std::fs::read_to_string(example_path).expect("failed to read fixture example.md");
    ParsedDocument::from_markdown(&markdown)
        .expect("failed to parse fixture example markdown")
        .fields()
        .clone()
}

#[test]
fn defaults_parity_all_fixture_quills() {
    for fixture in fixture_names() {
        let quill = load_quill(fixture);
        let schema = build_schema_from_config(&quill.config).expect("failed to build schema");

        let old = extract_defaults_from_schema(&schema);
        let new = quill.config.defaults();

        assert_eq!(old, new, "defaults parity mismatch for fixture {fixture}");
    }
}

#[test]
fn examples_parity_all_fixture_quills() {
    for fixture in fixture_names() {
        let quill = load_quill(fixture);
        let schema = build_schema_from_config(&quill.config).expect("failed to build schema");

        let old = extract_examples_from_schema(&schema);
        let new = quill.config.examples();

        assert_eq!(old, new, "examples parity mismatch for fixture {fixture}");
    }
}

#[test]
fn coerce_parity_all_fixture_examples() {
    for fixture in fixture_names() {
        let quill = load_quill(fixture);
        let schema = build_schema_from_config(&quill.config).expect("failed to build schema");
        let fields = load_example_fields(fixture);

        let old = coerce_document(&schema, &fields);
        let new = quill
            .config
            .coerce(&fields)
            .expect("native coercion should succeed for fixture example");

        assert_eq!(old, new, "coerce parity mismatch for fixture {fixture}");
    }
}
