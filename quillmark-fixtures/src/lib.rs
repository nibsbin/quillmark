use std::path::{Path, PathBuf};

/// Get the path to a resource file in the fixtures
pub fn resource_path(name: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).join("resources").join(name)
}

/// Get the path to tonguetoquill-collection quills
pub fn quills_path(name: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .join("resources")
        .join("tonguetoquill-collection")
        .join("quills")
        .join(name)
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
