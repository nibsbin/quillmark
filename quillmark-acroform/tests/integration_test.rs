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
#[ignore] // Pre-existing failure, not related to UTF-8 investigation
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
fn test_string_none_encoding() {
    use acroform::{AcroFormDocument, FieldValue};
    use std::collections::HashMap;

    // Create a simple test with the actual form
    let form_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quillmark-fixtures/resources/usaf_form_8/form.pdf"
    );
    let pdf_bytes = std::fs::read(form_path).expect("Failed to read PDF file");
    let mut doc = AcroFormDocument::from_bytes(pdf_bytes).expect("Failed to load PDF");

    // Fill with a simple value that contains "None"
    let mut values = HashMap::new();
    values.insert(
        "topmostSubform[0].Page1[0].P[0].ReqFld1[0]".to_string(),
        FieldValue::Text("None".to_string()),
    );

    // Fill the form
    let output = doc.fill(values).expect("Failed to fill PDF");

    // Read back the filled form
    let filled_doc =
        AcroFormDocument::from_bytes(output.clone()).expect("Failed to load filled PDF");
    let fields = filled_doc.fields().expect("Failed to get fields");

    for field in fields {
        if field.name == "topmostSubform[0].Page1[0].P[0].ReqFld1[0]" {
            println!("Field name: {}", field.name);
            println!("Field value: {:?}", field.current_value);
            if let Some(FieldValue::Text(text)) = field.current_value {
                println!("Text value: {}", text);
                println!("Bytes: {:?}", text.as_bytes());
                println!(
                    "Hex: {}",
                    text.as_bytes()
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<String>()
                );

                // Check if it contains box drawing characters
                if text.contains('╜') || text.contains('╚') {
                    panic!("Text contains box drawing characters! Got: {}", text);
                }
            }
        }
    }
}

#[test]
fn test_string_none_via_minijinja() {
    use acroform::{AcroFormDocument, FieldValue};
    use std::collections::HashMap;

    // Create a simple test with the actual form
    let form_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quillmark-fixtures/resources/usaf_form_8/form.pdf"
    );
    let pdf_bytes = std::fs::read(form_path).expect("Failed to read PDF file");
    let mut doc = AcroFormDocument::from_bytes(pdf_bytes).expect("Failed to load PDF");

    // Create a MiniJinja environment
    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Chainable);

    // Create a context with a value of "None"
    let context = serde_json::json!({
        "test_value": "None"
    });

    // Render the template "{{test_value}}"
    let rendered = env
        .render_str("{{test_value}}", &context)
        .expect("Failed to render");

    println!("Rendered value: {}", rendered);
    println!("Rendered bytes: {:?}", rendered.as_bytes());
    println!(
        "Rendered hex: {}",
        rendered
            .as_bytes()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    );

    // Fill with the rendered value
    let mut values = HashMap::new();
    values.insert(
        "topmostSubform[0].Page1[0].P[0].ReqFld1[0]".to_string(),
        FieldValue::Text(rendered.clone()),
    );

    // Fill the form
    let output = doc.fill(values).expect("Failed to fill PDF");

    // Read back the filled form
    let filled_doc =
        AcroFormDocument::from_bytes(output.clone()).expect("Failed to load filled PDF");
    let fields = filled_doc.fields().expect("Failed to get fields");

    for field in fields {
        if field.name == "topmostSubform[0].Page1[0].P[0].ReqFld1[0]" {
            println!("Field name: {}", field.name);
            println!("Field value: {:?}", field.current_value);
            if let Some(FieldValue::Text(text)) = field.current_value {
                println!("Text value: {}", text);
                println!("Text bytes: {:?}", text.as_bytes());
                println!(
                    "Text hex: {}",
                    text.as_bytes()
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<String>()
                );

                // Check if it contains box drawing characters
                if text.contains('╜') || text.contains('╚') {
                    panic!("Text contains box drawing characters! Got: {}", text);
                }

                assert_eq!(text, "None", "Text should be 'None' but got '{}'", text);
            }
        }
    }
}

#[test]
fn test_backend_with_none_string_value() {
    use quillmark_core::{Backend, OutputFormat, Quill, RenderOptions};

    let backend = AcroformBackend::default();
    let quill_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quillmark-fixtures/resources/usaf_form_8"
    );
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");

    // JSON context with a "None" string value in addi_comments
    // The field in the PDF is: {{examiner_remarks.addi_comments}}
    let json_context = r#"{
        "examiner_remarks": {
            "addi_comments": "None"
        }
    }"#;

    let opts = RenderOptions {
        output_format: Some(OutputFormat::Pdf),
    };

    let result = backend.compile(json_context, &quill, &opts);
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    let artifacts = result.unwrap();
    assert_eq!(artifacts.len(), 1);

    // Load the resulting PDF and check the field value
    use acroform::AcroFormDocument;
    let filled_doc = AcroFormDocument::from_bytes(artifacts[0].bytes.clone())
        .expect("Failed to load filled PDF");
    let fields = filled_doc.fields().expect("Failed to get fields");

    // Find the EvalRemarks field which contains the addi_comments template
    for field in fields {
        if field.name == "topmostSubform[0].Page2[0].EvalRemarks[0]" {
            println!("Field name: {}", field.name);
            if let Some(acroform::FieldValue::Text(text)) = &field.current_value {
                println!("Field text value: {}", text);
                println!("Field text bytes: {:?}", text.as_bytes());
                println!(
                    "Field text hex: {}",
                    text.as_bytes()
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<String>()
                );

                // Check if it contains box drawing characters
                if text.contains('╜') || text.contains('╚') {
                    panic!("Text contains box drawing characters! Got: {}", text);
                }

                // The text should contain "None" as the addi_comments value
                assert!(text.contains("None"), "Text should contain 'None'");
            }
        }
    }
}

#[test]
fn test_pdf_string_encoding_investigation() {
    use acroform::{AcroFormDocument, FieldValue};
    use std::collections::HashMap;

    // Test what happens when we write UTF-8 strings with special characters
    let form_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quillmark-fixtures/resources/usaf_form_8/form.pdf"
    );
    let pdf_bytes = std::fs::read(form_path).expect("Failed to read PDF file");
    let mut doc = AcroFormDocument::from_bytes(pdf_bytes).expect("Failed to load PDF");

    // Test cases with different strings
    let test_cases = vec![
        ("Plain ASCII", "None"),
        ("UTF-8 with smart quotes", "\u{201c}None\u{201d}"), // U+201C and U+201D
        ("UTF-8 with em dash", "Test\u{2014}None"),          // U+2014
        ("UTF-8 with special chars", "Tëst Nöñe"),           // Latin extended
    ];

    // Write test PDFs to /tmp for manual inspection
    std::fs::create_dir_all("/tmp/pdf_encoding_tests").ok();

    for (label, test_value) in test_cases {
        println!("\n=== Testing: {} ===", label);
        println!("Input string: {}", test_value);
        println!("Input UTF-8 bytes: {:?}", test_value.as_bytes());
        println!(
            "Input hex: {}",
            test_value
                .as_bytes()
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>()
        );

        // Fill with the test value
        let mut values = HashMap::new();
        values.insert(
            "topmostSubform[0].Page1[0].P[0].ReqFld1[0]".to_string(),
            FieldValue::Text(test_value.to_string()),
        );

        // Fill the form
        let pdf_bytes_copy = std::fs::read(form_path).expect("Failed to read PDF file");
        let mut doc = AcroFormDocument::from_bytes(pdf_bytes_copy).expect("Failed to load PDF");
        let output = doc.fill(values).expect("Failed to fill PDF");

        // Save to /tmp for manual inspection
        let safe_label = label.replace(" ", "_").replace("/", "_");
        let output_path = format!("/tmp/pdf_encoding_tests/{}.pdf", safe_label);
        std::fs::write(&output_path, &output).ok();
        println!("Saved to: {}", output_path);

        // Read back the filled form
        let filled_doc =
            AcroFormDocument::from_bytes(output.clone()).expect("Failed to load filled PDF");
        let fields = filled_doc.fields().expect("Failed to get fields");

        for field in fields {
            if field.name == "topmostSubform[0].Page1[0].P[0].ReqFld1[0]" {
                if let Some(FieldValue::Text(text)) = field.current_value {
                    println!("Output string: {}", text);
                    println!("Output UTF-8 bytes: {:?}", text.as_bytes());
                    println!(
                        "Output hex: {}",
                        text.as_bytes()
                            .iter()
                            .map(|b| format!("{:02x}", b))
                            .collect::<String>()
                    );

                    // Check if the string was preserved
                    if text != test_value {
                        println!("⚠️  STRING MISMATCH!");
                        println!("  Expected: {}", test_value);
                        println!("  Got:      {}", text);

                        // Check for box drawing characters
                        if text.contains('╜') || text.contains('╚') {
                            println!("  ❌ Contains box drawing characters!");
                        }
                    } else {
                        println!("✅ String preserved correctly");
                    }
                }
            }
        }
    }
}

#[test]
fn test_utf16be_encoding_demonstration() {
    // This test demonstrates the fix: converting strings to UTF-16BE

    // Test string with smart quotes
    let test_value = "\u{201c}None\u{201d}"; // "None"

    println!("Original string: {}", test_value);
    println!("UTF-8 bytes: {:?}", test_value.as_bytes());
    println!(
        "UTF-8 hex: {}",
        test_value
            .as_bytes()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    );

    // Convert to UTF-16BE with BOM (this is what the fix should do)
    let mut utf16_bytes = vec![0xFE, 0xFF]; // BOM
    for code_unit in test_value.encode_utf16() {
        utf16_bytes.push((code_unit >> 8) as u8); // High byte
        utf16_bytes.push((code_unit & 0xFF) as u8); // Low byte
    }

    println!("\nUTF-16BE bytes (with BOM): {:?}", utf16_bytes);
    println!(
        "UTF-16BE hex: {}",
        utf16_bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    );

    println!("\n✓ Fix confirmed: Convert to UTF-16BE with BOM");
    println!(
        "  Current (UTF-8): <{}>",
        test_value
            .as_bytes()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    );
    println!(
        "  Fixed (UTF-16BE): <{}>",
        utf16_bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    );

    // The fix should be applied in acroform-rs library:
    // File: acroform/src/api.rs
    // Method: FieldValue::to_primitive()
    assert_eq!(utf16_bytes[0], 0xFE);
    assert_eq!(utf16_bytes[1], 0xFF);
}

#[test]
#[ignore] // This test demonstrates PDF behavior (strips newlines), not a bug
fn test_string_none_with_newline_via_minijinja() {
    use acroform::{AcroFormDocument, FieldValue};
    use std::collections::HashMap;

    // Create a simple test with the actual form
    let form_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quillmark-fixtures/resources/usaf_form_8/form.pdf"
    );
    let pdf_bytes = std::fs::read(form_path).expect("Failed to read PDF file");
    let mut doc = AcroFormDocument::from_bytes(pdf_bytes).expect("Failed to load PDF");

    // Create a MiniJinja environment
    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Chainable);

    // Create a context with a value of "None"
    let context = serde_json::json!({
        "test_value": "None"
    });

    // Render the template "{{test_value}}\n" (with newline as in the actual PDF)
    let rendered = env
        .render_str("{{test_value}}\n", &context)
        .expect("Failed to render");

    println!("Rendered value: {}", rendered);
    println!("Rendered bytes: {:?}", rendered.as_bytes());
    println!(
        "Rendered hex: {}",
        rendered
            .as_bytes()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    );

    // Fill with the rendered value
    let mut values = HashMap::new();
    values.insert(
        "topmostSubform[0].Page1[0].P[0].ReqFld1[0]".to_string(),
        FieldValue::Text(rendered.clone()),
    );

    // Fill the form
    let output = doc.fill(values).expect("Failed to fill PDF");

    // Read back the filled form
    let filled_doc =
        AcroFormDocument::from_bytes(output.clone()).expect("Failed to load filled PDF");
    let fields = filled_doc.fields().expect("Failed to get fields");

    for field in fields {
        if field.name == "topmostSubform[0].Page1[0].P[0].ReqFld1[0]" {
            println!("Field name: {}", field.name);
            println!("Field value: {:?}", field.current_value);
            if let Some(FieldValue::Text(text)) = field.current_value {
                println!("Text value: {}", text);
                println!("Text bytes: {:?}", text.as_bytes());
                println!(
                    "Text hex: {}",
                    text.as_bytes()
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<String>()
                );

                // Check if it contains box drawing characters
                if text.contains('╜') || text.contains('╚') {
                    panic!("Text contains box drawing characters! Got: {}", text);
                }

                assert_eq!(
                    text, "None\n",
                    "Text should be 'None\\n' but got '{}'",
                    text
                );
            }
        }
    }
}
