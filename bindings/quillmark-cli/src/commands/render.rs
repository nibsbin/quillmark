use crate::errors::{CliError, Result};
use crate::output::{derive_glue_output_path, derive_output_path, OutputWriter};
use clap::Parser;
use quillmark::{ParsedDocument, Quill, Quillmark};
use quillmark_core::OutputFormat;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
pub struct RenderArgs {
    /// Path to markdown file with YAML frontmatter
    #[arg(value_name = "MARKDOWN_FILE")]
    markdown_file: PathBuf,

    /// Path to quill directory (overrides QUILL frontmatter field)
    #[arg(short, long, value_name = "PATH")]
    quill: Option<PathBuf>,

    /// Output file path (default: derived from input filename)
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Output format: pdf, svg, txt
    #[arg(short, long, value_name = "FORMAT", default_value = "pdf")]
    format: String,

    /// Write output to stdout instead of file
    #[arg(long)]
    stdout: bool,

    /// Only process glue template, don't render final output
    #[arg(long)]
    glue_only: bool,

    /// Show detailed processing information
    #[arg(short, long)]
    verbose: bool,

    /// Suppress all non-error output
    #[arg(short, long)]
    quiet: bool,
}

pub fn execute(args: RenderArgs) -> Result<()> {
    // Validate markdown file exists
    if !args.markdown_file.exists() {
        return Err(CliError::InvalidArgument(format!(
            "Markdown file not found: {}",
            args.markdown_file.display()
        )));
    }

    if args.verbose {
        println!("Reading markdown from: {}", args.markdown_file.display());
    }

    // Read markdown file
    let markdown = fs::read_to_string(&args.markdown_file)?;

    // Parse markdown
    let parsed = ParsedDocument::from_markdown(&markdown)
        .map_err(|e| CliError::Quillmark(anyhow::anyhow!("Failed to parse markdown: {}", e)))?;

    if args.verbose {
        println!("Markdown parsed successfully");
    }

    // Determine quill path
    let quill_path = if let Some(ref path) = args.quill {
        path.clone()
    } else {
        // Try to get QUILL field from frontmatter
        let quill_tag = parsed.quill_tag();

        // Check if a QUILL field was specified (not the default "__default__")
        if quill_tag == "__default__" {
            return Err(CliError::InvalidArgument(
                "No QUILL field in frontmatter and --quill not specified".to_string(),
            ));
        }

        // If QUILL field is a path, use it directly
        let quill_candidate = PathBuf::from(quill_tag);
        if quill_candidate.exists() && quill_candidate.is_dir() {
            quill_candidate
        } else {
            // Otherwise, try to find it relative to markdown file
            let markdown_dir = args
                .markdown_file
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."));
            markdown_dir.join(quill_tag)
        }
    };

    // Validate quill path exists
    if !quill_path.exists() {
        return Err(CliError::InvalidArgument(format!(
            "Quill directory not found: {}",
            quill_path.display()
        )));
    }

    if args.verbose {
        println!("Loading quill from: {}", quill_path.display());
    }

    // Load quill
    let quill = Quill::from_path(quill_path.clone())
        .map_err(|e| CliError::Quillmark(anyhow::anyhow!("Failed to load quill: {}", e)))?;

    if args.verbose {
        println!("Quill loaded: {}", quill.name);
    }

    // Create engine and workflow
    let engine = Quillmark::new();
    let workflow = engine
        .workflow_from_quill(&quill)
        .map_err(|e| CliError::Quillmark(anyhow::anyhow!("Failed to create workflow: {}", e)))?;

    if args.verbose {
        println!("Workflow created for backend: {}", workflow.backend_id());
    }

    // Handle glue-only mode
    if args.glue_only {
        if args.verbose {
            println!("Processing glue template...");
        }

        let glued = workflow
            .process_glue(&parsed)
            .map_err(|e| CliError::Quillmark(anyhow::anyhow!("Failed to process glue: {}", e)))?;

        let glued_bytes = glued.into_bytes();

        // Determine output path
        let output_path = args
            .output
            .unwrap_or_else(|| derive_glue_output_path(&args.markdown_file));

        let writer = OutputWriter::new(args.stdout, Some(output_path), args.quiet);
        writer.write(&glued_bytes)?;

        return Ok(());
    }

    // Parse output format
    let output_format = match args.format.to_lowercase().as_str() {
        "pdf" => OutputFormat::Pdf,
        "svg" => OutputFormat::Svg,
        "txt" => OutputFormat::Txt,
        _ => {
            return Err(CliError::InvalidArgument(format!(
                "Invalid output format: {}. Must be one of: pdf, svg, txt",
                args.format
            )));
        }
    };

    if args.verbose {
        println!("Rendering to format: {:?}", output_format);
    }

    // Render
    let result = workflow
        .render(&parsed, Some(output_format))
        .map_err(|e| CliError::Quillmark(anyhow::anyhow!("Failed to render: {}", e)))?;

    // Display warnings if any
    if !result.warnings.is_empty() && !args.quiet {
        eprintln!("Warnings:");
        for warning in &result.warnings {
            eprintln!("  - {}", warning.message);
        }
    }

    // Get the first artifact (there should only be one for single format render)
    let artifact = result.artifacts.first().ok_or_else(|| {
        CliError::Quillmark(anyhow::anyhow!(
            "No artifacts produced from rendering"
        ))
    })?;

    // Determine output path
    let output_path = if args.stdout {
        None
    } else {
        Some(
            args.output
                .unwrap_or_else(|| derive_output_path(&args.markdown_file, &args.format)),
        )
    };

    let writer = OutputWriter::new(args.stdout, output_path, args.quiet);
    writer.write(&artifact.bytes)?;

    if args.verbose && !args.quiet {
        println!("Rendering completed successfully");
    }

    Ok(())
}
