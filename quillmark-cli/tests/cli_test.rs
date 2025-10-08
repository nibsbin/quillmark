use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_cli_help() {
    let output = Command::new("cargo")
        .args(["run", "-p", "quillmark-cli", "--", "--help"])
        .output()
        .expect("Failed to run CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Render Markdown to PDF"));
    assert!(stdout.contains("--quill-path"));
    assert!(stdout.contains("<MARKDOWN>"));
}

#[test]
fn test_cli_missing_quill_error() {
    let temp_dir = TempDir::new().unwrap();
    let markdown_path = temp_dir.path().join("test.md");
    fs::write(&markdown_path, "# Test").unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "quillmark-cli",
            "--",
            markdown_path.to_str().unwrap(),
            "--quill-path",
            "/nonexistent/quill",
        ])
        .output()
        .expect("Failed to run CLI");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to load quill") || stderr.contains("Quill.toml not found"));
}

#[test]
fn test_cli_missing_markdown_error() {
    let fixtures_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("quillmark-fixtures/resources/taro");

    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "quillmark-cli",
            "--",
            "/nonexistent/file.md",
            "--quill-path",
            fixtures_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run CLI");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("No such file or directory") || stderr.contains("not found"));
}

#[test]
fn test_cli_basic_rendering() {
    let temp_dir = TempDir::new().unwrap();
    let markdown_path = temp_dir.path().join("test.md");

    // Get the fixtures path relative to the test
    let fixtures_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("quillmark-fixtures/resources/taro");

    // Write simple markdown without quill tag
    let markdown_content =
        "---\nauthor: Test\nice_cream: Vanilla\ntitle: Test\n---\n\nTest content";
    fs::write(&markdown_path, markdown_content).unwrap();

    let output_path = temp_dir.path().join("output.pdf");
    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "quillmark-cli",
            "--",
            markdown_path.to_str().unwrap(),
            "--quill-path",
            fixtures_path.to_str().unwrap(),
            "--output",
            output_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run CLI");

    // Check that it succeeded
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check that PDF was created
    assert!(output_path.exists(), "Output PDF was not created");
}

#[test]
fn test_cli_warning_on_mismatched_quill_tag() {
    let temp_dir = TempDir::new().unwrap();
    let markdown_path = temp_dir.path().join("test.md");

    // Get the fixtures path
    let fixtures_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("quillmark-fixtures/resources/usaf_memo");

    // Write markdown with a different quill tag
    let markdown_content =
        "---\nQUILL: different_quill\nfrom: Test\nto: Test\nsubject: Test\n---\n\nTest content";
    fs::write(&markdown_path, markdown_content).unwrap();

    let output_path = temp_dir.path().join("output.pdf");
    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "quillmark-cli",
            "--",
            markdown_path.to_str().unwrap(),
            "--quill-path",
            fixtures_path.to_str().unwrap(),
            "--output",
            output_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run CLI");

    // Check that it succeeded
    assert!(output.status.success(), "CLI failed");

    // Check that warning was shown
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Warning: Markdown specifies quill 'different_quill'"),
        "Expected warning about mismatched quill tag"
    );

    // Check that PDF was created despite warning
    assert!(output_path.exists(), "Output PDF was not created");
}
