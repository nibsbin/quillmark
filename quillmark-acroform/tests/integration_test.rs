use acroform::AcroFormDocument;
use quillmark_acroform::AcroformBackend;

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
