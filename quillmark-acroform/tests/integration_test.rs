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
    let filled_doc = AcroFormDocument::from_bytes(output.clone()).expect("Failed to load filled PDF");
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
    let rendered = env.render_str("{{test_value}}", &context).expect("Failed to render");
    
    println!("Rendered value: {}", rendered);
    println!("Rendered bytes: {:?}", rendered.as_bytes());
    println!(
        "Rendered hex: {}",
        rendered.as_bytes()
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
    let filled_doc = AcroFormDocument::from_bytes(output.clone()).expect("Failed to load filled PDF");
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
    let rendered = env.render_str("{{test_value}}\n", &context).expect("Failed to render");
    
    println!("Rendered value: {}", rendered);
    println!("Rendered bytes: {:?}", rendered.as_bytes());
    println!(
        "Rendered hex: {}",
        rendered.as_bytes()
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
    let filled_doc = AcroFormDocument::from_bytes(output.clone()).expect("Failed to load filled PDF");
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
                
                assert_eq!(text, "None\n", "Text should be 'None\\n' but got '{}'", text);
            }
        }
    }
}
