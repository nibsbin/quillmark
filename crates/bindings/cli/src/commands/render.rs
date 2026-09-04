use crate::commands::load_quill;
use crate::errors::{CliError, Result};
use crate::output::{derive_output_path, page_output_path, write_output};
use clap::Parser;
use quillmark::{Document, Quillmark};
use quillmark_core::{OutputFormat, RenderOptions};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
pub struct RenderArgs {
    /// Path to quill directory
    #[arg(value_name = "QUILL_PATH")]
    quill: PathBuf,

    /// Path to markdown file with card-yaml blocks
    #[arg(value_name = "MARKDOWN_FILE")]
    markdown_file: Option<PathBuf>,

    /// Output file path (default: derived from input filename)
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Output format: pdf, svg, png
    #[arg(short, long, value_name = "FORMAT", default_value = "pdf")]
    format: String,

    /// Write output to stdout instead of file
    #[arg(long)]
    stdout: bool,

    /// Show detailed processing information
    #[arg(short, long)]
    verbose: bool,

    /// Suppress all non-error output
    #[arg(long)]
    quiet: bool,

    /// Output intermediate JSON data to file
    #[arg(long, value_name = "DATA_FILE")]
    output_data: Option<PathBuf>,
}

// Progress chatter goes to stderr: under `--stdout` the artifact owns stdout,
// and a `--verbose` line there would land inside the emitted PDF.
pub fn execute(args: RenderArgs) -> Result<()> {
    if args.verbose {
        eprintln!("Loading quill from: {}", args.quill.display());
    }

    let quill = load_quill(&args.quill)?;

    if args.verbose {
        eprintln!("Quill loaded: {}", quill.name());
    }

    let (parsed, parse_warnings, markdown_path_for_output) =
        if let Some(ref markdown_path) = args.markdown_file {
            if !markdown_path.exists() {
                return Err(CliError::InvalidArgument(format!(
                    "Markdown file not found: {}",
                    markdown_path.display()
                )));
            }

            if args.verbose {
                eprintln!("Reading markdown from: {}", markdown_path.display());
            }

            let markdown = fs::read_to_string(markdown_path)?;
            let output = Document::parse(&markdown)?;

            if args.verbose {
                eprintln!("Markdown parsed successfully");
            }
            (
                output.document,
                output.warnings,
                Some(markdown_path.clone()),
            )
        } else {
            // No input file: the seed renders without any caller-supplied values.
            if args.verbose {
                eprintln!("Using seeded document from quill");
            }
            (quill.seed_document(), Vec::new(), None)
        };

    if args.verbose {
        eprintln!("Render-ready quill for backend: {}", quill.backend_id());
    }

    let output_format = args
        .format
        .parse::<OutputFormat>()
        .map_err(|e| CliError::InvalidArgument(e.to_string()))?;

    if args.verbose {
        eprintln!("Rendering to format: {:?}", output_format);
    }

    if let Some(data_path) = args.output_data {
        let json_data = quill.compile_data(&parsed).map_err(CliError::Render)?;
        let f = std::fs::File::create(&data_path).map_err(|e| {
            CliError::Io(std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to create data output file '{}': {}",
                    data_path.display(),
                    e
                ),
            ))
        })?;
        serde_json::to_writer_pretty(f, &json_data).map_err(|e| {
            CliError::Io(std::io::Error::other(format!(
                "Failed to write JSON data: {}",
                e
            )))
        })?;
        if args.verbose && !args.quiet {
            eprintln!("JSON data written to: {}", data_path.display());
        }
    }

    let engine = Quillmark::new();
    let mut result = engine.render(
        &quill,
        &parsed,
        &RenderOptions::default().with_output_format(output_format),
    )?;

    // One channel for downstream tooling.
    result.warnings.splice(0..0, parse_warnings);

    if !result.warnings.is_empty() && !args.quiet {
        crate::errors::print_warnings(&result.warnings);
    }

    if result.artifacts.is_empty() {
        return Err(CliError::InvalidArgument(
            "No artifacts produced from rendering".to_string(),
        ));
    }

    if args.stdout {
        if result.artifacts.len() > 1 {
            return Err(CliError::InvalidArgument(format!(
                "{} renders {} pages, one artifact each, and --stdout carries one; \
                 drop --stdout to write the pages as files",
                output_format,
                result.artifacts.len()
            )));
        }
        write_output(true, None, args.quiet, &result.artifacts[0].bytes)?;
    } else {
        let output_path = args.output.unwrap_or_else(|| {
            if let Some(ref path) = markdown_path_for_output {
                derive_output_path(path, output_format.as_str())
            } else {
                PathBuf::from(format!("example.{}", output_format))
            }
        });
        if let [artifact] = result.artifacts.as_slice() {
            write_output(false, Some(&output_path), args.quiet, &artifact.bytes)?;
        } else {
            // Every page numbered, page one included: an unnumbered file beside
            // numbered ones reads as the whole document.
            for (i, artifact) in result.artifacts.iter().enumerate() {
                let path = page_output_path(&output_path, i + 1);
                write_output(false, Some(&path), args.quiet, &artifact.bytes)?;
            }
        }
    }

    if args.verbose && !args.quiet {
        eprintln!("Rendering completed successfully");
    }

    Ok(())
}
