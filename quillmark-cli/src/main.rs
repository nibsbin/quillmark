use anyhow::{Context, Result};
use quillmark::{OutputFormat, ParsedDocument, Quill, Quillmark};
use quillmark_core::error::print_errors;
use std::env;
use std::fs;
use std::path::Path;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: quillmark-cli <quill_path> <markdown_file>");
        eprintln!();
        eprintln!("Arguments:");
        eprintln!("  <quill_path>      Path to the quill template directory");
        eprintln!("  <markdown_file>   Path to the markdown file to render");
        eprintln!();
        eprintln!("Output:");
        eprintln!("  Generates <markdown_file>.pdf in the same directory as the markdown file");
        std::process::exit(1);
    }

    let quill_path = &args[1];
    let markdown_file = &args[2];

    // Load the quill
    let quill = Quill::from_path(quill_path)
        .map_err(|e| anyhow::anyhow!("Failed to load quill from '{}': {}", quill_path, e))?;

    // Read the markdown file
    let markdown_content = fs::read_to_string(markdown_file)
        .with_context(|| format!("Failed to read markdown file '{}'", markdown_file))?;

    // Parse the markdown
    let parsed = ParsedDocument::from_markdown(&markdown_content)
        .with_context(|| "Failed to parse markdown document")?;

    // Create engine and workflow
    let engine = Quillmark::new();
    let workflow = engine
        .workflow_from_quill(&quill)
        .with_context(|| format!("Failed to create workflow for quill '{}'", quill.name))?;

    // Render to PDF
    let result = workflow.render(&parsed, Some(OutputFormat::Pdf));

    match result {
        Ok(render_result) => {
            if let Some(artifact) = render_result.artifacts.first() {
                // Determine output path
                let input_path = Path::new(markdown_file);
                let output_path = input_path.with_extension("pdf");

                // Write PDF to file
                fs::write(&output_path, &artifact.bytes).with_context(|| {
                    format!("Failed to write PDF to '{}'", output_path.display())
                })?;

                println!("Generated PDF: {}", output_path.display());
                Ok(())
            } else {
                anyhow::bail!("No artifacts were generated during rendering");
            }
        }
        Err(e) => {
            eprintln!("Error rendering document:");
            print_errors(&e);
            std::process::exit(1);
        }
    }
}
