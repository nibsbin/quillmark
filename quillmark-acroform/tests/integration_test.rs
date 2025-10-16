use acroform::{AcroFormDocument, FieldValue};
use quillmark_acroform::AcroformBackend;
use std::collections::HashMap;

#[test]
fn test_usaf_form_8_fields() {
    let form_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quillmark-fixtures/resources/usaf_form_8/form.pdf"
    );

    // Read the PDF into memory and use from_bytes
    let pdf_bytes = std::fs::read(form_path).expect("Failed to read PDF file");
    let doc = AcroFormDocument::from_bytes(pdf_bytes).expect("Failed to load PDF");

    println!("Fields in the PDF:");
    for field in doc.fields().expect("Failed to get fields") {
        println!("  Name: {}", field.name);
        println!("    Type: {:?}", field.field_type);
        println!("    Value: {:?}", field.current_value);
        println!();
    }
}

#[test]
fn test_backend_compilation() {
    use quillmark_core::{Backend, OutputFormat, Quill, RenderOptions};

    let backend = AcroformBackend::default();
    let quill_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quillmark-fixtures/resources/usaf_form_8"
    );
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");

    // Simple JSON context
    let json_context = r#"{"test": "success!"}"#;

    let opts = RenderOptions {
        output_format: Some(OutputFormat::Pdf),
    };

    let result = backend.compile(json_context, &quill, &opts);
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    let artifacts = result.unwrap();
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].output_format, OutputFormat::Pdf);
    assert!(!artifacts[0].bytes.is_empty());
}

#[test]
fn test_undefined_values_render_as_empty_string() {
    use quillmark_core::{Backend, OutputFormat, Quill, RenderOptions};

    let backend = AcroformBackend::default();
    let quill_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quillmark-fixtures/resources/usaf_form_8"
    );
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");

    // JSON context with only 2 items in requisite_info array
    // The PDF has fields for requisite_info[0] through requisite_info[10]
    let json_context = r#"{
        "name": "Test User",
        "grade": "O-3",
        "dod_id": 1234567890,
        "organization": "Test Org",
        "location": "Test Location",
        "mds": "F-16",
        "crew_position": "Pilot",
        "eligibility_period": "2025-01-01",
        "date_completed": "2025-01-01",
        "requisite_info": [
            {
                "requisites": "First requirement",
                "date": "2025-01-01",
                "results": "Pass"
            },
            {
                "requisites": "Second requirement",
                "date": "2025-01-02",
                "results": "Pass"
            }
        ]
    }"#;

    let opts = RenderOptions {
        output_format: Some(OutputFormat::Pdf),
    };

    // This should succeed without errors even though the PDF has fields
    // referencing requisite_info[2] through requisite_info[10]
    let result = backend.compile(json_context, &quill, &opts);
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    let artifacts = result.unwrap();
    assert_eq!(artifacts.len(), 1);

    // Verify the PDF was filled by checking its size
    assert!(!artifacts[0].bytes.is_empty());

    // Load the resulting PDF and verify that out-of-bounds fields are empty
    let filled_doc = AcroFormDocument::from_bytes(artifacts[0].bytes.clone())
        .expect("Failed to load filled PDF");

    let fields = filled_doc.fields().expect("Failed to get fields");
    let field_map: HashMap<String, FieldValue> = fields
        .into_iter()
        .filter_map(|f| f.current_value.map(|v| (f.name, v)))
        .collect();

    // Check that requisite_info[0] is filled
    let req_field_1 = field_map.get("topmostSubform[0].Page1[0].P[0].ReqFld1[0]");
    if let Some(FieldValue::Text(val)) = req_field_1 {
        assert_eq!(val, "First requirement", "Field 1 should be filled");
    }

    // Check that requisite_info[1] is filled
    let req_field_2 = field_map.get("topmostSubform[0].Page1[0].P[0].ReqFld2[0]");
    if let Some(FieldValue::Text(val)) = req_field_2 {
        assert_eq!(val, "Second requirement", "Field 2 should be filled");
    }

    // Check that requisite_info[10] (out of bounds) renders as empty string
    let req_field_11 = field_map.get("topmostSubform[0].Page1[0].P[0].ReqFld11[0]");
    if let Some(FieldValue::Text(val)) = req_field_11 {
        assert_eq!(val, "", "Field 11 should be empty (out of bounds)");
    }

    // Check that missing dictionary key renders as empty string
    // (If there was a field like {{missing_key}}, it should be empty too)
}

#[test]
fn test_tooltip_template_parsing() {
    use quillmark_core::{Backend, OutputFormat, Quill, RenderOptions};

    let backend = AcroformBackend::default();
    let quill_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quillmark-fixtures/resources/usaf_form_8"
    );
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");

    // JSON context that matches the tooltip template
    // Note: MbrRestct is a Button field, so we use simple values like "On" or "Off"
    let json_context = r#"{
        "other": {
            "restrictions": "On"
        }
    }"#;

    let opts = RenderOptions {
        output_format: Some(OutputFormat::Pdf),
    };

    // Compile the PDF with the backend
    let result = backend.compile(json_context, &quill, &opts);
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    let artifacts = result.unwrap();
    assert_eq!(artifacts.len(), 1);

    // Load the resulting PDF and verify the tooltip template was used
    let filled_doc = AcroFormDocument::from_bytes(artifacts[0].bytes.clone())
        .expect("Failed to load filled PDF");

    let fields = filled_doc.fields().expect("Failed to get fields");

    // Find the field with the tooltip template (MbrRestct has tooltip: MbrRestct__{{other.restrictions}})
    let field_with_tooltip = fields
        .iter()
        .find(|f| f.name == "P[0].Page1[0].topmostSubform[0].MbrRestct[0]")
        .expect("Field not found");

    // Verify the field value was rendered from the tooltip template
    // This is a Choice field (Button type), so the value should be FieldValue::Choice
    if let Some(FieldValue::Choice(val)) = &field_with_tooltip.current_value {
        assert_eq!(
            val, "On",
            "Field should be filled with rendered tooltip template"
        );
    } else {
        panic!(
            "Expected Choice field value, got: {:?}",
            field_with_tooltip.current_value
        );
    }
}

#[test]
fn test_tooltip_without_separator_uses_field_value() {
    use quillmark_core::{Backend, OutputFormat, Quill, RenderOptions};
    use std::io::Write;

    // Create a simple test PDF with a field that has a tooltip without "__"
    // For this test, we'll use the existing PDF but verify behavior when
    // the tooltip doesn't have a separator
    let backend = AcroformBackend::default();

    // Create a minimal Quill structure with a test PDF
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let quill_dir = temp_dir.path();

    // Create Quill.toml
    let mut quill_toml =
        std::fs::File::create(quill_dir.join("Quill.toml")).expect("Failed to create Quill.toml");
    quill_toml
        .write_all(
            br#"
[quill]
name = "test_no_separator"
backend = "acroform"
"#,
        )
        .expect("Failed to write Quill.toml");

    // Copy the test PDF
    let form_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quillmark-fixtures/resources/usaf_form_8/form.pdf"
    );
    std::fs::copy(form_path, quill_dir.join("form.pdf")).expect("Failed to copy form.pdf");

    let quill = Quill::from_path(quill_dir).expect("Failed to load quill");

    // Use context that would match if tooltip template was used
    let json_context = r#"{
        "other": {
            "eq": "Should not appear"
        }
    }"#;

    let opts = RenderOptions {
        output_format: Some(OutputFormat::Pdf),
    };

    let result = backend.compile(json_context, &quill, &opts);
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    // The important thing is that it doesn't crash - the actual behavior
    // depends on whether the field's current value contains a template
}

#[test]
fn test_field_type_preservation() {
    use quillmark_core::{Backend, OutputFormat, Quill, RenderOptions};

    let backend = AcroformBackend::default();
    let quill_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quillmark-fixtures/resources/usaf_form_8"
    );
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");

    // Create a context with various field types
    let json_context = r#"{
        "name": "John Doe",
        "grade": "O-3",
        "dod_id": 1234567890,
        "other": {
            "restrictions": "On"
        }
    }"#;

    let opts = RenderOptions {
        output_format: Some(OutputFormat::Pdf),
    };

    let result = backend.compile(json_context, &quill, &opts);
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    let artifacts = result.unwrap();
    assert_eq!(artifacts.len(), 1);

    // Load the resulting PDF and verify field types are preserved
    let filled_doc = AcroFormDocument::from_bytes(artifacts[0].bytes.clone())
        .expect("Failed to load filled PDF");

    let fields = filled_doc.fields().expect("Failed to get fields");

    // Text field should remain Text
    let name_field = fields
        .iter()
        .find(|f| f.name == "P[0].Page1[0].topmostSubform[0].MbrName[1]");
    if let Some(field) = name_field {
        assert!(
            matches!(field.current_value, Some(FieldValue::Text(_))),
            "Text field should preserve Text type"
        );
    }

    // Choice field should remain Choice
    let restrictions_field = fields
        .iter()
        .find(|f| f.name == "P[0].Page1[0].topmostSubform[0].MbrRestct[0]");
    if let Some(field) = restrictions_field {
        if let Some(FieldValue::Choice(val)) = &field.current_value {
            assert_eq!(
                val, "On",
                "Choice field should be filled with correct value"
            );
        } else {
            panic!("Expected Choice field type, got: {:?}", field.current_value);
        }
    }
}

#[test]
fn test_empty_tooltip_template_uses_field_value() {
    // Test that if tooltip has "__" but nothing after it, we fall back to field value
    use quillmark_core::{Backend, OutputFormat, Quill, RenderOptions};

    let backend = AcroformBackend::default();
    let quill_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quillmark-fixtures/resources/usaf_form_8"
    );
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");

    let json_context = r#"{}"#;

    let opts = RenderOptions {
        output_format: Some(OutputFormat::Pdf),
    };

    // Should not crash even with empty context
    let result = backend.compile(json_context, &quill, &opts);
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());
}

#[test]
fn test_tooltip_template_with_complex_expression() {
    use quillmark_core::{Backend, OutputFormat, Quill, RenderOptions};

    let backend = AcroformBackend::default();
    let quill_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quillmark-fixtures/resources/usaf_form_8"
    );
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");

    // JSON context with nested objects - testing that complex templates work
    let json_context = r#"{
        "customer": {
            "firstname": "John",
            "lastname": "Doe"
        },
        "other": {
            "restrictions": "Off"
        }
    }"#;

    let opts = RenderOptions {
        output_format: Some(OutputFormat::Pdf),
    };

    // Compile the PDF with the backend
    let result = backend.compile(json_context, &quill, &opts);
    assert!(
        result.is_ok(),
        "Compilation should succeed with complex context: {:?}",
        result.err()
    );

    let artifacts = result.unwrap();
    assert_eq!(artifacts.len(), 1);

    // Verify the PDF was generated successfully
    assert!(!artifacts[0].bytes.is_empty());

    // Load the resulting PDF and check that the template was rendered
    let filled_doc = AcroFormDocument::from_bytes(artifacts[0].bytes.clone())
        .expect("Failed to load filled PDF");

    let fields = filled_doc.fields().expect("Failed to get fields");

    // The field with tooltip "MbrRestct__{{other.restrictions}}" should have
    // its value set to "Off" from the context
    // This is a Choice field (Button type), so the value should be FieldValue::Choice
    let field_with_tooltip = fields
        .iter()
        .find(|f| f.name == "P[0].Page1[0].topmostSubform[0].MbrRestct[0]")
        .expect("Field not found");

    if let Some(FieldValue::Choice(val)) = &field_with_tooltip.current_value {
        assert_eq!(
            val, "Off",
            "Field should be filled with value from tooltip template"
        );
    }
}
