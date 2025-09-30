use quillmark::{OutputFormat, Quill, Workflow};
use quillmark_fixtures::resource_path;
use quillmark_typst::TypstBackend;
use std::fs;

#[test]
fn test_with_existing_fixture() {
    // Use the existing usaf-memo fixture
    let quill_path = resource_path("usaf-memo");
    println!("Testing with fixture at: {:?}", quill_path);

    // Load the sample frontmatter demo markdown
    let sample_markdown_path = resource_path("frontmatter_demo.md");
    let markdown =
        fs::read_to_string(&sample_markdown_path).expect("Failed to read sample markdown");

    // Create engine
    let backend = Box::new(TypstBackend::default());
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    let engine = Workflow::new(backend, quill).expect("Failed to create engine");

    println!("Created engine for quill: {}", engine.quill_name());

    // Test rendering
    let result = engine
        .render(&markdown, Some(OutputFormat::Pdf))
        .expect("Failed to render");

    assert!(!result.artifacts.is_empty());
    assert_eq!(result.artifacts[0].output_format, OutputFormat::Pdf);

    println!(
        "Successfully rendered {} bytes of PDF output",
        result.artifacts[0].bytes.len()
    );
    println!("Fixture integration test passed!");
}
