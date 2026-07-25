//! # `$quill` Reference Enforcement Tests
//!
//! A document's `$quill: name@selector` reference is checked against the loaded
//! quill at render time (and in `dry_run`). The document may be perfectly valid,
//! but rendering it against the wrong quill — a different *name*, or a `version`
//! outside the selector — is a footgun, so it is a hard error
//! (`quill::name_mismatch` / `quill::version_mismatch`), never a warning.

use quillmark::Document;
use quillmark_core::RenderError;
use std::fs;
use tempfile::TempDir;

// The reject-path tests drive `engine.render`, which resolves the quill's
// declared `typst` backend before the reference check runs — without the
// feature they would fail on `engine::backend_not_found` instead. `dry_run`
// needs no backend, so the accept-path tests build unconditionally.
#[cfg(feature = "typst")]
use quillmark::Quillmark;
#[cfg(feature = "typst")]
use quillmark_core::{OutputFormat, RenderOptions};

/// Write a minimal typst quill named `test_quill` at the given version.
fn make_quill(temp_dir: &TempDir, version: &str) -> std::path::PathBuf {
    let quill_path = temp_dir.path().join("test_quill");
    fs::create_dir_all(&quill_path).unwrap();
    fs::write(
        quill_path.join("Quill.yaml"),
        format!(
            "quill:\n  name: \"test_quill\"\n  version: \"{}\"\n  backend: \"typst\"\n  description: \"Test\"\n\ntypst:\n  plate_file: plate.typ\n",
            version
        ),
    )
    .unwrap();
    fs::write(quill_path.join("plate.typ"), "Content").unwrap();
    quill_path
}

#[cfg(feature = "typst")]
fn render_ref(
    quill_path: &std::path::Path,
    quill_ref: &str,
) -> Result<quillmark_core::RenderResult, RenderError> {
    let engine = Quillmark::new();
    let quill = quillmark::quill_from_path(quill_path).expect("from_path failed");
    let markdown = format!(
        "~~~card-yaml\n$quill: {}\n$kind: main\n~~~\n\n# Content\n",
        quill_ref
    );
    let doc = Document::parse(&markdown).expect("parse failed").document;
    engine.render(
        &quill,
        &doc,
        &RenderOptions {
            output_format: Some(OutputFormat::Pdf),
            ..Default::default()
        },
    )
}

/// `dry_run` a document referencing `quill_ref` against the quill at
/// `quill_path`. Proves selector acceptance without driving a Typst compile —
/// the render seam itself is covered by the reject-path tests.
fn dry_run_ref(quill_path: &std::path::Path, quill_ref: &str) -> Result<(), RenderError> {
    let quill = quillmark::quill_from_path(quill_path).expect("from_path failed");
    let markdown = format!(
        "~~~card-yaml\n$quill: {}\n$kind: main\n~~~\n\n# Content\n",
        quill_ref
    );
    let doc = Document::parse(&markdown).expect("parse failed").document;
    quill.dry_run(&doc)
}

/// The single code carried by a quill-mismatch error (the check emits exactly one).
#[cfg(feature = "typst")]
fn mismatch_code(err: &RenderError) -> Option<&str> {
    err.diagnostics().first().and_then(|d| d.code.as_deref())
}

#[test]
#[cfg(feature = "typst")]
fn version_out_of_selector_is_a_hard_error() {
    let temp_dir = TempDir::new().unwrap();
    let quill_path = make_quill(&temp_dir, "3.0.0");

    // Document pins `@2`; loaded quill is 3.0.0 → incompatible → render fails.
    let err = render_ref(&quill_path, "test_quill@2").expect_err("render should fail");
    assert_eq!(mismatch_code(&err), Some("quill::version_mismatch"));
}

#[test]
#[cfg(feature = "typst")]
fn name_mismatch_is_a_hard_error() {
    let temp_dir = TempDir::new().unwrap();
    let quill_path = make_quill(&temp_dir, "3.0.0");

    // Name differs — render fails on the name, and the version is left
    // unevaluated (a selector against a differently-named quill is moot).
    let err = render_ref(&quill_path, "other_quill@2").expect_err("render should fail");
    assert_eq!(mismatch_code(&err), Some("quill::name_mismatch"));
}

#[test]
fn exact_selector_match_accepts() {
    let temp_dir = TempDir::new().unwrap();
    let quill_path = make_quill(&temp_dir, "2.1.0");

    dry_run_ref(&quill_path, "test_quill@2.1.0").expect("selector should be accepted");
}
