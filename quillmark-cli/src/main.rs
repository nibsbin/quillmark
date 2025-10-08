use clap::Parser;
use quillmark::{OutputFormat, ParsedDocument, Quill, Quillmark};
use quillmark_core::error::print_errors;
use std::path::PathBuf;

/// Minimal CLI for testing Quillmark packages locally
#[derive(Parser, Debug)]
#[command(name = "quillmark-cli")]
#[command(about = "Render Markdown to PDF using Quillmark Typst quills", long_about = None)]
struct Args {
    /// Path to the quill directory
    #[arg(short, long)]
    quill: PathBuf,

    /// Path to the markdown file to render
    #[arg(short, long)]
    markdown: PathBuf,

    /// Output PDF file path (defaults to output.pdf)
    #[arg(short, long, default_value = "output.pdf")]
    output: PathBuf,
}

fn main() {
    let args = Args::parse();

    if let Err(e) = run(args) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run(args: Args) -> anyhow::Result<()> {
    // Load the quill from the specified directory
    let quill = Quill::from_path(&args.quill)
        .map_err(|e| anyhow::anyhow!("Failed to load quill: {}", e))?;

    // Read the markdown file
    let markdown = std::fs::read_to_string(&args.markdown)?;

    // Parse the markdown
    let parsed = ParsedDocument::from_markdown(&markdown)?;

    // Create engine and workflow
    let engine = Quillmark::new();
    let workflow = engine.workflow_from_quill(&quill)?;

    // Render to PDF
    let result = match workflow.render(&parsed, Some(OutputFormat::Pdf)) {
        Ok(result) => result,
        Err(e) => {
            print_errors(&e);
            return Err(anyhow::anyhow!("Rendering failed"));
        }
    };

    // Write the PDF to the output file
    if let Some(artifact) = result.artifacts.first() {
        std::fs::write(&args.output, &artifact.bytes)?;
        println!("PDF written to: {}", args.output.display());
    } else {
        anyhow::bail!("No PDF artifact generated");
    }

    // Print any warnings
    for warning in result.warnings {
        eprintln!("Warning: {}", warning.message);
    }

    Ok(())
}
