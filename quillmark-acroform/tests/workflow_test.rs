use quillmark::{OutputFormat, ParsedDocument, Quillmark};

#[test]
fn test_acroform_workflow_e2e() {
    // Create engine with acroform backend
    let mut engine = Quillmark::new();

    // Load the usaf_form_8 quill
    let quill_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quillmark-fixtures/resources/usaf_form_8"
    );
    let quill = quillmark::Quill::from_path(quill_path).expect("Failed to load quill");
    engine.register_quill(quill);

    // Create workflow
    let workflow = engine
        .workflow_from_quill_name("usaf_form_8")
        .expect("Failed to create workflow");

    // Check that the backend is acroform
    assert_eq!(workflow.backend_id(), "acroform");

    // Parse the example markdown
    let markdown = r#"---
test: "Hello from Workflow!"
---
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    // Render
    let result = workflow.render(&parsed, Some(OutputFormat::Pdf));
    assert!(result.is_ok(), "Workflow render failed: {:?}", result.err());

    let render_result = result.unwrap();
    assert_eq!(render_result.artifacts.len(), 1);
    assert_eq!(render_result.artifacts[0].output_format, OutputFormat::Pdf);
    assert!(!render_result.artifacts[0].bytes.is_empty());

    println!(
        "Generated PDF size: {} bytes",
        render_result.artifacts[0].bytes.len()
    );
}
