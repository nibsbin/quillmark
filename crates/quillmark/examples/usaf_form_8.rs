use std::path::PathBuf;

use quillmark::{OutputFormat, ParsedDocument, Quillmark};
use quillmark_fixtures::{quills_path, write_example_output};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Path to the usaf_form_8 fixture
    let quill_path = quills_path("usaf_form_8");

    // Load quill
    let quill = quillmark::Quill::from_path(quill_path).expect("Failed to load quill");

    // Use the example template from the quill if present, otherwise use a small frontmatter
    let markdown = if let Some(example) = &quill.example {
        example.clone()
    } else {
        // Minimal frontmatter used in the test
        r#"---
test: "Hello from Example!"
---
"#
        .to_string()
    };

    // Parse the markdown
    let parsed = ParsedDocument::from_markdown(&markdown)?;

    // Create engine and register quill
    let mut engine = Quillmark::new();
    engine.register_quill(quill.clone())?;

    // Build workflow
    let workflow = engine.workflow(&quill).expect("Failed to create workflow");

    // Compose glue output (JSON)
    let glue_output = workflow.process_glue(&parsed)?;
    write_example_output("usaf_form_8_glue.json", glue_output.as_bytes())?;

    let output_dir = PathBuf::from("crates/fixtures/output/");

    println!(
        "Wrote glue output to examples output directory: {}",
        output_dir.join("usaf_form_8_glue.json").display()
    );

    // Render to PDF
    let result = workflow.render(&parsed, Some(OutputFormat::Pdf))?;
    if !result.artifacts.is_empty() {
        let pdf_bytes = &result.artifacts[0].bytes;
        write_example_output("usaf_form_8.pdf", pdf_bytes)?;
        let pdf_path = output_dir.join("usaf_form_8.pdf");
        println!(
            "Wrote rendered PDF to examples output directory: {}",
            pdf_path.display()
        );
    } else {
        println!("No artifacts produced by render");
    }

    Ok(())
}
