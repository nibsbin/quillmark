use anyhow::{Context, Result};
use clap::Parser;
use quillmark::{OutputFormat, ParsedDocument, Quill, Quillmark};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "quillmark-cli")]
#[command(about = "Render Markdown with Quillmark templates to PDF", long_about = None)]
struct Cli {
    /// Path to the quill template directory
    quill_path: PathBuf,

    /// Path to the markdown file to render
    markdown_file: PathBuf,

    /// Output PDF file path (defaults to input filename with .pdf extension)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load quill
    let quill = Quill::from_path(&cli.quill_path)
        .map_err(|e| anyhow::anyhow!("Failed to load quill from {:?}: {}", cli.quill_path, e))?;

    // Read markdown file
    let markdown = fs::read_to_string(&cli.markdown_file)
        .with_context(|| format!("Failed to read markdown file {:?}", cli.markdown_file))?;

    // Parse markdown
    let parsed = ParsedDocument::from_markdown(&markdown)
        .map_err(|e| anyhow::anyhow!("Failed to parse markdown: {}", e))?;

    // Create engine and workflow
    let engine = Quillmark::new();
    let workflow = engine
        .workflow_from_quill(&quill)
        .map_err(|e| anyhow::anyhow!("Failed to create workflow from quill: {}", e))?;

    // Render to PDF
    let result = workflow
        .render(&parsed, Some(OutputFormat::Pdf))
        .map_err(|e| anyhow::anyhow!("Failed to render document: {}", e))?;

    // Determine output path
    let output_path = cli.output.unwrap_or_else(|| {
        let mut path = cli.markdown_file.clone();
        path.set_extension("pdf");
        path
    });

    // Write output
    fs::write(&output_path, &result.artifacts[0].bytes)
        .with_context(|| format!("Failed to write output to {:?}", output_path))?;

    println!(
        "Successfully rendered {} bytes to {:?}",
        result.artifacts[0].bytes.len(),
        output_path
    );

    Ok(())
}
