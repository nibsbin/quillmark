use crate::errors::{CliError, Result};
use crate::output::{derive_glue_output_path, derive_output_path, OutputWriter};
use clap::Parser;
use quillmark::{ParsedDocument, Plate, Quillmark};
use quillmark_core::OutputFormat;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
pub struct RenderArgs {
    /// Path to markdown file with YAML frontmatter
    #[arg(value_name = "MARKDOWN_FILE")]
    markdown_file: Option<PathBuf>,

    /// Path to quill directory (overrides QUILL frontmatter field)
    #[arg(short, long, value_name = "PATH")]
    plate: Option<PathBuf>,

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
    #[arg(long)]
    quiet: bool,
}

pub fn execute(args: RenderArgs) -> Result<()> {
    // Determine if we have a markdown file or need to use example content
    let (parsed, quill, markdown_path_for_output) = if let Some(ref markdown_path) =
        args.markdown_file
    {
        // Validate markdown file exists
        if !markdown_path.exists() {
            return Err(CliError::InvalidArgument(format!(
                "Markdown file not found: {}",
                markdown_path.display()
            )));
        }

        if args.verbose {
            println!("Reading markdown from: {}", markdown_path.display());
        }

        // Read markdown file
        let markdown = fs::read_to_string(markdown_path)?;

        // Parse markdown
        let parsed = ParsedDocument::from_markdown(&markdown)?;

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
                let markdown_dir = markdown_path
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."));
                markdown_dir.join(quill_tag)
            }
        };

        // Validate quill path exists
        if !quill_path.exists() {
            return Err(CliError::InvalidArgument(format!(
                "Plate directory not found: {}",
                quill_path.display()
            )));
        }

        if args.verbose {
            println!("Loading quill from: {}", quill_path.display());
        }

        // Load quill
        let quill = Plate::from_path(quill_path.clone())?;

        if args.verbose {
            println!("Plate loaded: {}", quill.name);
        }

        (parsed, quill, Some(markdown_path.clone()))
    } else {
        // No markdown file provided, must have --quill
        let quill_path = args.quill.clone().ok_or_else(|| {
            CliError::InvalidArgument("Must provide either a markdown file or --quill".to_string())
        })?;

        // Validate quill path exists
        if !quill_path.exists() {
            return Err(CliError::InvalidArgument(format!(
                "Plate directory not found: {}",
                quill_path.display()
            )));
        }

        if args.verbose {
            println!("Loading quill from: {}", quill_path.display());
        }

        // Load quill
        let quill = Plate::from_path(quill_path.clone())?;

        if args.verbose {
            println!("Plate loaded: {}", quill.name);
        }

        // Get example content
        let markdown = quill.example.clone().ok_or_else(|| {
            CliError::InvalidArgument(format!(
                "Plate '{}' does not have example content",
                quill.name
            ))
        })?;

        if args.verbose {
            println!("Using example content from quill");
        }

        // Parse markdown
        let parsed = ParsedDocument::from_markdown(&markdown)?;

        if args.verbose {
            println!("Example markdown parsed successfully");
        }

        (parsed, quill, None)
    };

    // Create engine and workflow
    let engine = Quillmark::new();
    let workflow = engine.workflow(&quill)?;

    if args.verbose {
        println!("Workflow created for backend: {}", workflow.backend_id());
    }

    // Handle glue-only mode
    if args.glue_only {
        if args.verbose {
            println!("Processing glue template...");
        }

        let glued = workflow.process_glue(&parsed)?;

        let glued_bytes = glued.into_bytes();

        // Determine output path
        let output_path = args.output.unwrap_or_else(|| {
            if let Some(ref path) = markdown_path_for_output {
                derive_glue_output_path(path)
            } else {
                PathBuf::from("example_glue.typ")
            }
        });

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
    let result = workflow.render(&parsed, Some(output_format))?;

    // Display warnings if any
    if !result.warnings.is_empty() && !args.quiet {
        crate::errors::print_warnings(&result.warnings);
    }

    // Get the first artifact (there should only be one for single format render)
    let artifact = result.artifacts.first().ok_or_else(|| {
        CliError::InvalidArgument("No artifacts produced from rendering".to_string())
    })?;

    // Determine output path
    let output_path = if args.stdout {
        None
    } else {
        Some(args.output.unwrap_or_else(|| {
            if let Some(ref path) = markdown_path_for_output {
                derive_output_path(path, &args.format)
            } else {
                PathBuf::from(format!("example.{}", args.format))
            }
        }))
    };

    let writer = OutputWriter::new(args.stdout, output_path, args.quiet);
    writer.write(&artifact.bytes)?;

    if args.verbose && !args.quiet {
        println!("Rendering completed successfully");
    }

    Ok(())
}
