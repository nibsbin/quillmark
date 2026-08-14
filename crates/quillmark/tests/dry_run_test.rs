use quillmark::Document;
use std::fs;
use tempfile::TempDir;

fn make_test_quill_path(temp_dir: &TempDir) -> std::path::PathBuf {
    let quill_path = temp_dir.path().join("test_quill");
    fs::create_dir_all(&quill_path).unwrap();
    fs::write(
        quill_path.join("Quill.yaml"),
        "quill:\n  name: \"test_quill\"\n  version: \"1.0\"\n  backend: \"typst\"\n  description: \"Test\"\n\ntypst:\n  plate_file: plate.typ\n\nmain:\n  fields:\n    title:\n      type: \"string\"\n    author:\n      type: \"string\"\n      default: \"\"\n",
    ).unwrap();
    fs::write(quill_path.join("plate.typ"), "Title: {{ title }}").unwrap();
    quill_path
}

#[test]
fn test_dry_run_tolerates_must_fill_marker() {
    // The marker surfaces as a `validate` warning, not a render error.
    let temp_dir = TempDir::new().unwrap();
    let quill_path = make_test_quill_path(&temp_dir);

    let quill = quillmark::quill_from_path(&quill_path).expect("from_path failed");

    let markdown =
        "~~~card-yaml\n$quill: test_quill\n$kind: main\ntitle: !must_fill\nauthor: Test\n~~~\n\n# Content\n";
    let parsed = Document::parse(markdown).expect("parse failed").document;

    let result = quill.dry_run(&parsed);
    assert!(
        result.is_ok(),
        "dry_run should tolerate a !must_fill placeholder (blank-filled): {:?}",
        result
    );
}

