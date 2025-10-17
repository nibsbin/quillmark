use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

use quillmark_wasm::Quillmark;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

// A minimal JSON fixture that represents a very small quill
const SMALL_QUILL_JSON: &str = r#"{
  "Quill.toml": { "contents": "[Quill]\nname = \"test-quill\"\nbackend = \"typst\"\nglue = \"glue.typ\"\ndescription = \"Test quill for WASM bindings\"\n" },
  "glue.typ": { "contents": "= Title\n\nThis is a test." },
  "content.md": { "contents": "---\ntitle: Test\n---\n\n# Hello" }
}"#;

#[wasm_bindgen_test]
fn engine_register_and_render() {
    // Create engine
    let mut engine = Quillmark::new();

    // Register quill
    engine
        .register_quill("test-quill", JsValue::from_str(SMALL_QUILL_JSON))
        .expect("register failed");

    // Call render_glue on a small markdown
    let glue_out = engine
        .render_glue("test-quill", "---\ntitle: Glue\n---\n\n# X")
        .expect("render_glue failed");
    assert!(glue_out.len() > 0);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_usaf_form_8_glue_output() {
    use quillmark_core::{ParsedDocument, Quill};
    use std::path::Path;

    // Load the usaf_form_8 quill from filesystem
    let quill_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quillmark-fixtures/resources/usaf_form_8"
    );
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");

    // Load the example markdown
    let markdown_path = Path::new(quill_path).join("usaf_form_8.md");
    let markdown = std::fs::read_to_string(markdown_path).expect("Failed to read markdown");

    // Parse the markdown
    let parsed = ParsedDocument::from_markdown(&markdown).expect("Failed to parse markdown");

    // Create a Quillmark engine
    let mut native_engine = quillmark::Quillmark::new();
    native_engine.register_quill(quill.clone());

    // Create workflow and get glue output
    let workflow = native_engine
        .workflow_from_quill_name("usaf_form_8")
        .expect("Failed to create workflow");

    let glue_output = workflow
        .process_glue_parsed(&parsed)
        .expect("Failed to process glue");

    // Parse the glue output as JSON to verify it has data
    let json: serde_json::Value =
        serde_json::from_str(&glue_output).expect("Failed to parse glue output as JSON");

    // Verify that examinee data is present and not null
    assert!(json.get("examinee").is_some(), "examinee field missing");
    let examinee = json.get("examinee").unwrap();
    assert!(examinee.is_object(), "examinee should be an object");

    // Check that first name is present and not null
    let first = examinee.get("first");
    assert!(first.is_some(), "examinee.first is missing");
    assert!(!first.unwrap().is_null(), "examinee.first is null");
    assert_eq!(
        first.unwrap().as_str(),
        Some("Phillip"),
        "examinee.first should be 'Phillip'"
    );

    // Check other fields
    assert_eq!(examinee.get("last").and_then(|v| v.as_str()), Some("Fry"));
    assert_eq!(examinee.get("middle").and_then(|v| v.as_str()), Some("J."));
    assert_eq!(examinee.get("grade").and_then(|v| v.as_str()), Some("SrA"));

    // Print glue output for debugging
    println!(
        "Glue output (first 500 chars): {}",
        &glue_output[..glue_output.len().min(500)]
    );
}

/// Helper function to convert a filesystem Quill to JSON format for WASM
#[cfg(not(target_arch = "wasm32"))]
fn quill_to_json(quill_path: &str) -> String {
    use serde_json::json;
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;

    fn build_file_tree(path: &Path, base_path: &Path) -> serde_json::Value {
        let mut result = HashMap::new();

        if path.is_file() {
            // Read file contents
            let contents = fs::read(path).expect("Failed to read file");

            // Check if it's likely a text file
            let is_text = path
                .extension()
                .and_then(|e| e.to_str())
                .map_or(true, |ext| {
                    matches!(
                        ext,
                        "toml" | "md" | "txt" | "typ" | "tex" | "html" | "css" | "js"
                    )
                });

            if is_text {
                // Try to convert to UTF-8 string
                if let Ok(text) = String::from_utf8(contents.clone()) {
                    return json!({ "contents": text });
                }
            }

            // Binary file - return as byte array
            return json!({ "contents": contents });
        }

        // Directory - recursively build file tree
        for entry in fs::read_dir(path).expect("Failed to read directory") {
            let entry = entry.expect("Failed to read directory entry");
            let entry_path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files
            if name.starts_with('.') {
                continue;
            }

            result.insert(name, build_file_tree(&entry_path, base_path));
        }

        json!(result)
    }

    let path = Path::new(quill_path);
    let files = build_file_tree(path, path);

    let quill_json = json!({
        "files": files
    });

    serde_json::to_string_pretty(&quill_json).expect("Failed to serialize to JSON")
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_usaf_form_8_from_json_matches_from_path() {
    use quillmark_core::Quill;

    // Load quill from filesystem
    let quill_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quillmark-fixtures/resources/usaf_form_8"
    );
    let quill_from_path = Quill::from_path(quill_path).expect("Failed to load quill from path");

    // Convert to JSON and load again
    let quill_json = quill_to_json(quill_path);
    println!(
        "Quill JSON (first 1000 chars): {}",
        &quill_json[..quill_json.len().min(1000)]
    );

    let quill_from_json = Quill::from_json(&quill_json).expect("Failed to load quill from JSON");

    // Verify both quills have the same name
    assert_eq!(quill_from_path.name, quill_from_json.name);

    // Verify both have form.pdf
    let form_pdf_from_path = quill_from_path.files.get_file("form.pdf");
    let form_pdf_from_json = quill_from_json.files.get_file("form.pdf");
    assert!(
        form_pdf_from_path.is_some(),
        "form.pdf not found in quill from path"
    );
    assert!(
        form_pdf_from_json.is_some(),
        "form.pdf not found in quill from JSON"
    );
    assert_eq!(
        form_pdf_from_path.unwrap().len(),
        form_pdf_from_json.unwrap().len(),
        "form.pdf sizes differ"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_usaf_form_8_render_via_json_workflow() {
    use quillmark_core::{OutputFormat, ParsedDocument, Quill};
    use std::path::Path;

    // Load quill from JSON (simulating WASM workflow)
    let quill_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quillmark-fixtures/resources/usaf_form_8"
    );
    let quill_json = quill_to_json(quill_path);
    let quill = Quill::from_json(&quill_json).expect("Failed to load quill from JSON");

    // Load the example markdown
    let markdown_path = Path::new(quill_path).join("usaf_form_8.md");
    let markdown = std::fs::read_to_string(markdown_path).expect("Failed to read markdown");

    // Parse the markdown
    let parsed = ParsedDocument::from_markdown(&markdown).expect("Failed to parse markdown");

    // Create engine and register quill (simulating WASM)
    let mut engine = quillmark::Quillmark::new();
    engine.register_quill(quill.clone());

    // Create workflow
    let workflow = engine
        .workflow_from_quill_name("usaf_form_8")
        .expect("Failed to create workflow");

    // Get glue output
    let glue_output = workflow
        .process_glue_parsed(&parsed)
        .expect("Failed to process glue");

    // Parse and verify glue output has correct data
    let json: serde_json::Value =
        serde_json::from_str(&glue_output).expect("Failed to parse glue output as JSON");

    // Verify examinee data
    let examinee = json.get("examinee").expect("examinee field missing");
    assert_eq!(
        examinee.get("first").and_then(|v| v.as_str()),
        Some("Phillip")
    );
    assert_eq!(examinee.get("last").and_then(|v| v.as_str()), Some("Fry"));

    // Try to render to PDF
    let result = workflow
        .render(&parsed, Some(OutputFormat::Pdf))
        .expect("Failed to render PDF");

    assert!(!result.artifacts.is_empty(), "No artifacts produced");
    assert_eq!(result.artifacts[0].output_format, OutputFormat::Pdf);
    assert!(!result.artifacts[0].bytes.is_empty(), "PDF is empty");

    println!(
        "Successfully rendered PDF with {} bytes",
        result.artifacts[0].bytes.len()
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_usaf_form_8_with_minimal_markdown() {
    use quillmark_core::{OutputFormat, ParsedDocument, Quill};

    // Load quill from JSON (simulating WASM workflow)
    let quill_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quillmark-fixtures/resources/usaf_form_8"
    );
    let quill_json = quill_to_json(quill_path);
    let quill = Quill::from_json(&quill_json).expect("Failed to load quill from JSON");

    // Test with minimal markdown (just the QUILL tag, no data)
    let minimal_markdown = "---\nQUILL: usaf_form_8\n---\n";

    // Parse the markdown
    let parsed = ParsedDocument::from_markdown(minimal_markdown).expect("Failed to parse markdown");

    // Create engine and register quill
    let mut engine = quillmark::Quillmark::new();
    engine.register_quill(quill.clone());

    // Create workflow
    let workflow = engine
        .workflow_from_quill_name("usaf_form_8")
        .expect("Failed to create workflow");

    // Get glue output
    let glue_output = workflow
        .process_glue_parsed(&parsed)
        .expect("Failed to process glue");

    println!("Glue output with minimal markdown: {}", glue_output);

    // Parse glue output
    let json: serde_json::Value =
        serde_json::from_str(&glue_output).expect("Failed to parse glue output as JSON");

    // With minimal markdown, all fields should be present but empty or null
    // The body field should be empty
    assert_eq!(json.get("body").and_then(|v| v.as_str()), Some(""));

    // Try to render - this should produce a PDF with all blank fields
    let result = workflow
        .render(&parsed, Some(OutputFormat::Pdf))
        .expect("Failed to render PDF");

    assert!(!result.artifacts.is_empty(), "No artifacts produced");
    assert!(!result.artifacts[0].bytes.is_empty(), "PDF is empty");

    println!(
        "Successfully rendered PDF with minimal data: {} bytes",
        result.artifacts[0].bytes.len()
    );
}
