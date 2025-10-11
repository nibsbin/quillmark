use anyhow::{Context, Result};
use quillmark::{OutputFormat, ParsedDocument, Quill, Quillmark};
use quillmark_core::error::print_errors;
use std::env;
use std::fs;
use std::path::Path;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    // Parse arguments
    let mut output_glue = false;
    let mut positional_args = Vec::new();

    for arg in args.iter().skip(1) {
        if arg == "--output-glue" {
            output_glue = true;
        } else {
            positional_args.push(arg.as_str());
        }
    }

    if positional_args.len() != 2 {
        eprintln!("Usage: quillmark-cli [--output-glue] <quill_path> <markdown_file>");
        eprintln!();
        eprintln!("Arguments:");
        eprintln!("  <quill_path>      Path to the quill template directory");
        eprintln!("  <markdown_file>   Path to the markdown file to render");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --output-glue     Also output the rendered/composed glue template");
        eprintln!();
        eprintln!("Output:");
        eprintln!("  Generates <markdown_file>.pdf in the same directory as the markdown file");
        eprintln!("  With --output-glue, also generates <markdown_file>.glue.typ");
        std::process::exit(1);
    }

    let quill_path = positional_args[0];
    let markdown_file = positional_args[1];

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

    // Process glue if requested
    if output_glue {
        let glue_content = workflow
            .process_glue_parsed(&parsed)
            .with_context(|| "Failed to process glue template")?;

        let input_path = Path::new(markdown_file);
        let glue_path = input_path.with_extension("glue.typ");

        fs::write(&glue_path, glue_content.as_bytes())
            .with_context(|| format!("Failed to write glue output to '{}'", glue_path.display()))?;

        println!("Generated glue: {}", glue_path.display());
    }

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
