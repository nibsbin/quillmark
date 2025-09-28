use std::path::PathBuf;

use quillmark_typst::TypstBackend;
use quillmark::{render, RenderConfig};
use quillmark_core::test_context;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the Typst backend
    let backend = TypstBackend::new();

    // Define output directory
    let output_dir = std::path::PathBuf::from("output");
    std::fs::create_dir_all(&output_dir)?;

    // Path to quill template - should be at workspace level
    let examples_dir = test_context::examples_dir().map_err(|e| -> Box<dyn std::error::Error> { e })?;
    let quill_path = examples_dir.join("hello-quill");
    if !quill_path.exists() {
        return Err(format!("Quill path does not exist: {}", quill_path.display()).into());
    }

    // Load markdown
    let markdown_path: PathBuf = examples_dir.join("sample.md");

    println!("Markdown path: {}", markdown_path.display());
    let mark_content = std::fs::read_to_string(&markdown_path)
        .map_err(|e| format!("Failed to read markdown file: {}", e))?;

    let config = RenderConfig {
        backend: Box::new(backend),
        output_format: Some(quillmark_core::OutputFormat::Pdf),
        quill_path: quill_path
    };

    let pdf_output = render(&mark_content, &config)?;

    // Write output PDF
    let output_pdf_path = output_dir.join("hello-quill.pdf"); 
    std::fs::write(&output_pdf_path, pdf_output[0].bytes.clone())
        .map_err(|e| format!("Failed to write output PDF: {}", e))?;  

    // Output will be in output/hello-quill.pdf
    println!("Rendered output to {}/hello-quill.pdf", output_dir.display());


    Ok(())
}