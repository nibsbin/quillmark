use crate::commands::load_quill;
use crate::errors::Result;
use crate::output::write_file;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
pub struct BlueprintArgs {
    /// Path to quill directory
    #[arg(value_name = "QUILL_PATH")]
    quill: PathBuf,

    /// Output file path (optional)
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,
}

pub fn execute(args: BlueprintArgs) -> Result<()> {
    let quill = load_quill(&args.quill)?;

    let blueprint = quill.config().blueprint();

    if let Some(output_path) = args.output {
        write_file(&output_path, blueprint.as_bytes(), false)?;
    } else {
        println!("{}", blueprint);
    }

    Ok(())
}
