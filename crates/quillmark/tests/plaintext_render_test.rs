//! A `plaintext` field lowers through the typst backend with no backend-side
//! special case: it rides the same `contentMediaType` as richtext, so the
//! shared content-lowering path emits it, and its literal codec keeps markdown
//! delimiters verbatim.

#![cfg(feature = "typst")]

use quillmark::{Document, OutputFormat, Quillmark, RenderOptions};
use std::fs;
use tempfile::TempDir;

fn plaintext_quill(temp_dir: &TempDir) -> std::path::PathBuf {
    let quill_path = temp_dir.path().join("plain_quill");
    fs::create_dir_all(&quill_path).unwrap();
    fs::write(
        quill_path.join("Quill.yaml"),
        r#"quill:
  name: "plain_quill"
  version: "1.0"
  backend: "typst"
  description: "plaintext lowering"

main:
  body:
    enabled: false
  fields:
    subject:
      type: plaintext
      default: ""
"#,
    )
    .unwrap();
    // The backend pre-lowers content into the `data` dict; the plate just reads it.
    fs::write(
        quill_path.join("plate.typ"),
        "#import \"@local/quillmark-helper:0.1.0\": data\n= Doc\n#data.subject\n",
    )
    .unwrap();
    quill_path
}

#[test]
fn plaintext_field_lowers_through_typst_backend() {
    let temp_dir = TempDir::new().unwrap();
    let quill_path = plaintext_quill(&temp_dir);

    let engine = Quillmark::new();
    let quill = quillmark::quill_from_path(quill_path).expect("load quill");
    let md = "~~~card-yaml\n$quill: plain_quill\n$kind: main\n\
              subject: \"a *literal* subject with _no_ markup\"\n~~~\n";
    let parsed = Document::parse(md).expect("parse").document;

    let result = engine.render(
        &quill,
        &parsed,
        &RenderOptions::default().with_output_format(OutputFormat::Svg),
    );
    assert!(
        result.is_ok(),
        "plaintext field should lower and render through the typst backend, got: {:?}",
        result.err()
    );
}
