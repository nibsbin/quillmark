use crate::errors::Result;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Write `bytes` to stdout, or to `output_path` (creating parent directories),
/// announcing the destination unless `quiet`. Stdout wins when both are set;
/// neither set is an argument error.
pub fn write_output(
    use_stdout: bool,
    output_path: Option<&Path>,
    quiet: bool,
    bytes: &[u8],
) -> Result<()> {
    if use_stdout {
        io::stdout().write_all(bytes)?;
        return Ok(());
    }
    let Some(path) = output_path else {
        return Err(crate::errors::CliError::InvalidArgument(
            "No output path configured and stdout output not selected".to_string(),
        ));
    };
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(path, bytes)?;
    if !quiet {
        println!("Output written to: {}", path.display());
    }
    Ok(())
}

/// The input markdown path with its extension replaced by `format`.
pub fn derive_output_path(markdown_path: &Path, format: &str) -> PathBuf {
    let mut output = markdown_path.to_path_buf();
    output.set_extension(format);
    output
}

/// `path` with `-{page}` appended to its file stem: `out.svg` at page 2 is
/// `out-2.svg`. A path with no stem keeps the number as its whole name.
pub fn page_output_path(path: &Path, page: usize) -> PathBuf {
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned());
    let mut name = match stem {
        Some(stem) if !stem.is_empty() => format!("{stem}-{page}"),
        _ => page.to_string(),
    };
    if let Some(ext) = path.extension() {
        name.push('.');
        name.push_str(&ext.to_string_lossy());
    }
    path.with_file_name(name)
}
