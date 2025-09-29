use std::path::PathBuf;

use quillmark_typst::TypstBackend;
use quillmark::{QuillEngine, OutputFormat};
use quillmark_fixtures::{resource_path, write_example_output};


fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the Typst backend
    let backend = TypstBackend::new();

    // Path to quill template - use fixtures
    let quill_path = resource_path("hello-quill")?;
    if !quill_path.exists() {
        return Err(format!("Quill path does not exist: {}", quill_path.display()).into());
    }

    // Load markdown from fixtures
    let markdown_path: PathBuf = resource_path("sample.md")?;

    println!("Markdown path: {}", markdown_path.display());
    let mark_content = std::fs::read_to_string(&markdown_path)
        .map_err(|e| format!("Failed to read markdown file: {}", e))?;

    // Create QuillEngine with the backend and quill template
    let engine = QuillEngine::new(Box::new(backend), quill_path)?;

    // Render to PDF using the new API
    let pdf_output = engine.render_with_format(&mark_content, Some(OutputFormat::Pdf))?;

    // Write output PDF using fixtures utility
    let output_path = write_example_output("hello-quill", "hello-quill.pdf", &pdf_output[0].bytes)?;

    println!("Rendered output to {}", output_path.display());

    Ok(())
}