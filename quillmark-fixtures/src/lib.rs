use std::path::{Path, PathBuf};
use std::error::Error;

/// Get the path to a resource file in the fixtures
pub fn resource_path(name: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).join("resources").join(name)
}

/// Get the example output directory path
pub fn example_output_dir() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).join("output")
}

/// Write example output to the examples directory
pub fn write_example_output(name: &str, content: &[u8]) -> Result<(), std::io::Error> {
    use std::fs;

    let output_dir = example_output_dir();
    fs::create_dir_all(&output_dir)?;

    let output_path = output_dir.join(name);
    fs::write(output_path, content)?;

    Ok(())
}

/// List all available resource files
pub fn list_resources() -> Result<Vec<String>, std::io::Error> {
    use std::fs;

    let resources_dir = resource_path("");
    let entries = fs::read_dir(resources_dir)?;

    let mut resources = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        resources.push(name);
    }

    resources.sort();
    Ok(resources)
}

/// Demo helper that centralizes example plumbing.
///
/// It reads the given resource markdown, computes the quill path, then calls the
/// provided `runner` closure to perform backend-specific work. The `runner`
/// should return a tuple of (glue_bytes, output_bytes) which this helper will
/// write to the example output directory and print a short preview.
pub fn demo<F>(
    resource_name: &str,
    quill_dir: &str,
    glue_name: &str,
    output_name: &str,
    runner: F,
) -> Result<(), Box<dyn Error>>
where
    F: Fn(&str, &Path) -> Result<(Vec<u8>, Vec<u8>), Box<dyn Error>>,
{
    // Load the sample markdown
    let markdown = std::fs::read_to_string(resource_path(resource_name))?;

    // quill path (folder)
    let quill_path = resource_path(quill_dir);

    // run backend-specific logic provided by the caller
    let (glued_bytes, output_bytes) = runner(&markdown, &quill_path)?;

    // write outputs
    write_example_output(glue_name, &glued_bytes)?;
    println!(
        "Processed glue content preview: \n\n{}...\n",
        &String::from_utf8_lossy(&glued_bytes)[..std::cmp::min(500, glued_bytes.len())]
    );

    write_example_output(output_name, &output_bytes)?;

    println!(
        "Access files:\n- Glue: {}\n- Output: {}",
        example_output_dir().join(glue_name).display(),
        example_output_dir().join(output_name).display()
    );

    Ok(())
}
