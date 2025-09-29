use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

/// Errors that can occur when working with fixtures
#[derive(thiserror::Error, Debug)]
pub enum FixtureError {
    #[error("Failed to find workspace root: {0}")]
    WorkspaceNotFound(String),
    #[error("CARGO_TARGET_DIR not found")]
    TargetDirNotFound,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Resource not found: {path}")]
    ResourceNotFound { path: String },
}

/// Get the path to a fixture resource
/// 
/// # Arguments
/// * `relative_path` - Path relative to the fixtures resources directory
/// 
/// # Examples
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use quillmark_fixtures::resource_path;
/// let sample_md = resource_path("sample.md")?;
/// # Ok(())
/// # }
/// ```
pub fn resource_path(relative_path: &str) -> Result<PathBuf, FixtureError> {
    // Get the directory where this crate is located
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| FixtureError::WorkspaceNotFound("CARGO_MANIFEST_DIR not set".into()))?;
    
    let resources_dir = PathBuf::from(manifest_dir)
        .parent()
        .ok_or_else(|| FixtureError::WorkspaceNotFound("Cannot find parent directory".into()))?
        .join("quillmark-fixtures")
        .join("resources");
    
    let resource_path = resources_dir.join(relative_path);
    
    if !resource_path.exists() {
        return Err(FixtureError::ResourceNotFound { 
            path: resource_path.display().to_string() 
        });
    }
    
    Ok(resource_path)
}

/// Create and get an output directory for an example in target/examples/<example-name>/
/// 
/// This ensures all example outputs are written to the standard location
/// under CARGO_TARGET_DIR/examples/<example-name>/
/// 
/// # Arguments
/// * `example_name` - Name of the example (used as directory name)
/// 
/// # Examples
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use quillmark_fixtures::example_output_dir;
/// let output_dir = example_output_dir("hello-quill")?;
/// std::fs::write(output_dir.join("output.pdf"), b"PDF content")?;
/// # Ok(())
/// # }
/// ```
pub fn example_output_dir(example_name: &str) -> Result<PathBuf, FixtureError> {
    // First try CARGO_TARGET_DIR, then fall back to finding workspace root + target
    let target_dir = if let Ok(target_dir) = env::var("CARGO_TARGET_DIR") {
        PathBuf::from(target_dir)
    } else {
        find_workspace_root()?.join("target")
    };
    
    let examples_dir = target_dir.join("examples").join(example_name);
    
    // Create the directory if it doesn't exist
    fs::create_dir_all(&examples_dir)?;
    
    Ok(examples_dir)
}

/// Write content to a file in the example's output directory
/// 
/// This is a convenience function that combines example_output_dir with file writing
/// 
/// # Arguments
/// * `example_name` - Name of the example
/// * `filename` - Name of the file to write
/// * `content` - Content to write
/// 
/// # Examples
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use quillmark_fixtures::write_example_output;
/// write_example_output("hello-quill", "output.pdf", b"PDF content")?;
/// # Ok(())
/// # }
/// ```
pub fn write_example_output(
    example_name: &str, 
    filename: &str, 
    content: &[u8]
) -> Result<PathBuf, FixtureError> {
    let output_dir = example_output_dir(example_name)?;
    let file_path = output_dir.join(filename);
    
    fs::write(&file_path, content)?;
    
    Ok(file_path)
}

/// Find the workspace root by walking up the directory tree
fn find_workspace_root() -> Result<PathBuf, FixtureError> {
    let current_dir = env::current_dir()
        .map_err(|e| FixtureError::WorkspaceNotFound(format!("Cannot get current dir: {}", e)))?;
    
    let mut dir = current_dir.as_path();
    
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        
        if cargo_toml.exists() {
            // Check if this is a workspace root
            if let Ok(content) = fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") || content.contains("members") {
                    return Ok(dir.to_path_buf());
                }
            }
        }
        
        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }
    
    Err(FixtureError::WorkspaceNotFound(
        "Could not find workspace root with Cargo.toml containing [workspace]".into()
    ))
}

/// Get the workspace root examples directory (deprecated - use resource_path instead)
/// 
/// This function is kept for backward compatibility but is deprecated.
/// Use `resource_path("")` to get the resources directory instead.
#[deprecated(note = "Use resource_path(\"\") instead")]
pub fn examples_dir() -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    resource_path("")
        .map_err(|e| -> Box<dyn Error + Send + Sync> { Box::new(e) })
}

/// Create an output directory within the examples folder (deprecated)
/// 
/// This function is kept for backward compatibility but is deprecated.
/// Use `example_output_dir(subdir)` instead.
#[deprecated(note = "Use example_output_dir(subdir) instead")]
pub fn create_output_dir(subdir: &str) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    example_output_dir(subdir)
        .map_err(|e| -> Box<dyn Error + Send + Sync> { Box::new(e) })
}

/// Get a path to a file within the examples directory (deprecated)
/// 
/// This function is kept for backward compatibility but is deprecated.
/// Use `resource_path(relative_path)` instead.
#[deprecated(note = "Use resource_path(relative_path) instead")]
pub fn examples_path(relative_path: &str) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    resource_path(relative_path)
        .map_err(|e| -> Box<dyn Error + Send + Sync> { Box::new(e) })
}
