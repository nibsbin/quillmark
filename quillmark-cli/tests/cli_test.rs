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
    assert!(stdout.contains("--quill"));
    assert!(stdout.contains("--markdown"));
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
            "--quill",
            "/nonexistent/quill",
            "--markdown",
            markdown_path.to_str().unwrap(),
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
        .join("quillmark-fixtures/resources/usaf_memo");

    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "quillmark-cli",
            "--",
            "--quill",
            fixtures_path.to_str().unwrap(),
            "--markdown",
            "/nonexistent/file.md",
        ])
        .output()
        .expect("Failed to run CLI");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("No such file or directory") || stderr.contains("not found"));
}
