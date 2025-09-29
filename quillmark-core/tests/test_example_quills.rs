use quillmark_core::Quill;
use std::path::PathBuf;

#[test]
fn test_example_quills_with_toml() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let examples_path = PathBuf::from(&manifest_dir).parent().unwrap().join("examples");
    
    // Test hello-quill
    let hello_quill_path = examples_path.join("hello-quill");
    if hello_quill_path.exists() {
        let quill = Quill::from_path(&hello_quill_path)?;
        
        // Verify that metadata is loaded from quill.toml
        assert_eq!(quill.name, "hello-quill");
        assert_eq!(quill.metadata.get("version").and_then(|v| v.as_str()), Some("0.1.0"));
        assert_eq!(quill.metadata.get("description").and_then(|v| v.as_str()), 
                   Some("A simple hello world quill template demonstrating basic typography and layout features"));
        assert_eq!(quill.metadata.get("author").and_then(|v| v.as_str()), Some("QuillMark Team"));
        
        quill.validate()?;
    }
    
    // Test simple-quill
    let simple_quill_path = examples_path.join("simple-quill");
    if simple_quill_path.exists() {
        let quill = Quill::from_path(&simple_quill_path)?;
        
        // Verify that metadata is loaded from quill.toml
        assert_eq!(quill.name, "simple-quill");
        assert_eq!(quill.metadata.get("version").and_then(|v| v.as_str()), Some("1.0.0"));
        assert_eq!(quill.metadata.get("description").and_then(|v| v.as_str()), 
                   Some("A minimal quill template for simple document formatting"));
        assert_eq!(quill.metadata.get("author").and_then(|v| v.as_str()), Some("QuillMark Team"));
        
        quill.validate()?;
    }

    Ok(())
}