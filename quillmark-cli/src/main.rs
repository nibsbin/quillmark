use clap::Parser;
use quillmark::{OutputFormat, ParsedDocument, Quill, Quillmark};
use quillmark_core::error::print_errors;
use std::path::PathBuf;

/// Minimal CLI for testing Quillmark packages locally
#[derive(Parser, Debug)]
#[command(name = "quillmark-cli")]
#[command(about = "Render Markdown to PDF using Quillmark Typst quills", long_about = None)]
struct Args {
    /// Path to the markdown file to render
    markdown: PathBuf,

    /// Path to the quill directory (optional, uses 'quill' field from markdown frontmatter if not specified)
    #[arg(short, long)]
    quill: Option<PathBuf>,

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
    // Read the markdown file
    let markdown = std::fs::read_to_string(&args.markdown)?;

    // Parse the markdown
    let parsed = ParsedDocument::from_markdown(&markdown)?;

    // Create engine
    let engine = Quillmark::new();

    // Determine quill path: use --quill flag if provided, otherwise check frontmatter
    let quill_path = if let Some(quill_path) = args.quill {
        quill_path
    } else {
        // Try to get quill from frontmatter field
        if let Some(quill_value) = parsed.get_field("quill") {
            if let Some(quill_str) = quill_value.as_str() {
                PathBuf::from(quill_str)
            } else {
                anyhow::bail!("'quill' field in frontmatter must be a string");
            }
        } else if let Some(quill_tag) = parsed.quill_tag() {
            // Fallback to !quill tag if present
            PathBuf::from(quill_tag)
        } else {
            anyhow::bail!(
                "No quill specified. Either provide --quill flag or set 'quill' field in markdown frontmatter"
            )
        }
    };

    // Load the quill
    let quill = Quill::from_path(&quill_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to load quill from '{}': {}",
            quill_path.display(),
            e
        )
    })?;

    // Create workflow
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
