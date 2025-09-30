use std::fs;
use tempfile::TempDir;
use quillmark::{Quill, Workflow};
use quillmark_typst::TypstBackend;

#[test]
fn test_quill_automatic_validation_success() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");
    
    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(quill_path.join("quill.toml"), "[Quill]\nname = \"test\"\n").expect("Failed to write quill.toml");
    fs::write(quill_path.join("glue.typ"), "Test template").expect("Failed to write glue.typ");
    
    // This should succeed since glue.typ exists and validation passes
    let quill = Quill::from_path(quill_path).expect("Failed to create quill");
    
    assert_eq!(quill.name, "test-quill");
    assert_eq!(quill.glue_file, "glue.typ");
    assert_eq!(quill.glue_template, "Test template");
}

#[test]
fn test_quill_automatic_validation_failure() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");
    
    // Create quill directory but without the glue file
    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(quill_path.join("quill.toml"), "[Quill]\nname = \"test\"\n").expect("Failed to write quill.toml");
    // Note: No glue.typ file created
    
    // This should fail during automatic validation
    let result = Quill::from_path(quill_path);
    
    assert!(result.is_err(), "Expected validation to fail when glue file is missing");
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("glue.typ"), "Error should mention missing glue file");
}

#[test]
fn test_quill_automatic_validation_custom_glue_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");
    
    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    
    // Configure custom glue file
    let quill_toml = r#"
[Quill]
name = "test"
glue_file = "custom-glue.typ"
"#;
    fs::write(quill_path.join("quill.toml"), quill_toml).expect("Failed to write quill.toml");
    fs::write(quill_path.join("custom-glue.typ"), "Custom template").expect("Failed to write custom glue file");
    
    // This should succeed with custom glue file
    let quill = Quill::from_path(quill_path).expect("Failed to create quill");
    
    assert_eq!(quill.name, "test-quill"); 
    assert_eq!(quill.glue_file, "custom-glue.typ");
    assert_eq!(quill.glue_template, "Custom template");
}

#[test]
fn test_quill_automatic_validation_custom_glue_file_missing() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");
    
    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    
    // Configure custom glue file but don't create it
    let quill_toml = r#"
[Quill]
name = "test"
glue_file = "missing-glue.typ"
"#;
    fs::write(quill_path.join("quill.toml"), quill_toml).expect("Failed to write quill.toml");
    // Note: missing-glue.typ is not created
    
    // This should fail during automatic validation
    let result = Quill::from_path(quill_path);
    
    assert!(result.is_err(), "Expected validation to fail when custom glue file is missing");
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("missing-glue.typ"), "Error should mention missing custom glue file");
}

#[test]
fn test_workflow_no_longer_validates_manually() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");
    
    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(quill_path.join("quill.toml"), "[Quill]\nname = \"test\"\n").expect("Failed to write quill.toml");
    fs::write(quill_path.join("glue.typ"), "Test template").expect("Failed to write glue.typ");
    
    // Create a valid quill
    let quill = Quill::from_path(quill_path).expect("Failed to create quill");
    
    // Workflow::new should not need to validate again since the quill is already validated
    let backend = Box::new(TypstBackend::default());
    let workflow = Workflow::new(backend, quill).expect("Failed to create workflow");
    
    assert_eq!(workflow.quill_name(), "test-quill");
    assert_eq!(workflow.backend_id(), "typst");
}