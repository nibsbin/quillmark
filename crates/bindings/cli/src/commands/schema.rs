use crate::errors::{CliError, Result};
use clap::Parser;
use quillmark::Quill;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
pub struct SchemaArgs {
    /// Path to quill directory
    #[arg(value_name = "QUILL_PATH")]
    quill_path: PathBuf,

    /// Output file path (optional)
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,
}

pub fn execute(args: SchemaArgs) -> Result<()> {
    // Validate quill path exists
    if !args.quill_path.exists() {
        return Err(CliError::InvalidArgument(format!(
            "Quill directory not found: {}",
            args.quill_path.display()
        )));
    }

    // Load Quill
    let quill = Quill::from_path(&args.quill_path)?;

    // Serialize schema to JSON
    let schema_json = serde_json::to_string_pretty(&quill.schema)
        .map_err(|e| CliError::InvalidArgument(format!("Failed to serialize schema: {}", e)))?;

    // Output
    if let Some(output_path) = args.output {
        fs::write(&output_path, schema_json).map_err(CliError::Io)?;
    } else {
        println!("{}", schema_json);
    }

    Ok(())
}
