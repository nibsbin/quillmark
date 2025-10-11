use std::fs;
use std::path::PathBuf;
use std::process::Command;

const MIN_PDF_SIZE: usize = 1000;

#[test]
fn test_cli_renders_pdf() {
    let quill_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("quillmark-fixtures/resources/taro");

    let markdown_path = quill_path.join("taro.md");

    let temp_dir = tempfile::tempdir().unwrap();
    let temp_markdown = temp_dir.path().join("test.md");
    fs::copy(&markdown_path, &temp_markdown).unwrap();

    let binary_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("target/debug/quillmark-cli");

    let output = Command::new(&binary_path)
        .arg(&quill_path)
        .arg(&temp_markdown)
        .output()
        .expect("Failed to execute CLI");

    assert!(output.status.success(), "CLI should succeed");

    let pdf_path = temp_markdown.with_extension("pdf");
    assert!(pdf_path.exists(), "PDF file should be created");

    let pdf_content = fs::read(&pdf_path).unwrap();
    assert!(
        pdf_content.len() > MIN_PDF_SIZE,
        "PDF should have substantial content (>{} bytes)",
        MIN_PDF_SIZE
    );
    assert!(
        pdf_content.starts_with(b"%PDF"),
        "File should be a valid PDF"
    );
}

#[test]
fn test_cli_missing_args() {
    let binary_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("target/debug/quillmark-cli");

    let output = Command::new(&binary_path)
        .output()
        .expect("Failed to execute CLI");

    assert!(!output.status.success(), "CLI should fail with no args");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Usage:"),
        "Should show usage message on stderr"
    );
}

#[test]
fn test_cli_nonexistent_quill() {
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_markdown = temp_dir.path().join("test.md");
    fs::write(&temp_markdown, "# Test").unwrap();

    let nonexistent_quill = temp_dir.path().join("nonexistent_quill");

    let binary_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("target/debug/quillmark-cli");

    let output = Command::new(&binary_path)
        .arg(&nonexistent_quill)
        .arg(&temp_markdown)
        .output()
        .expect("Failed to execute CLI");

    assert!(!output.status.success(), "CLI should fail with bad quill");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Failed to load quill"),
        "Should show quill error"
    );
}
