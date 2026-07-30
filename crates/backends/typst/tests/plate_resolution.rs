//! The Typst backend resolves its own plate from the `typst.plate_file`
//! setting in the quill's config, reading the file from the quill's bundle.
//! Core reads no template at load time, so a missing plate is a render-time
//! (`open`) error rather than a load-time one.

use quillmark_core::Backend;
use quillmark_typst::TypstBackend;

mod common;
use common::quill;

const YAML: &str = "quill:\n  name: t\n  version: \"1.0\"\n  backend: typst\n  \
                    description: d\n\ntypst:\n  plate_file: plate.typ\n";

#[test]
fn plate_file_is_resolved_from_the_typst_section() {
    let q = quill(
        YAML,
        &[(
            "plate.typ",
            b"#set page(width: 100pt, height: 100pt)\n= Hi\n",
        )],
    );
    let session = TypstBackend
        .open(&q, &serde_json::json!({}))
        .expect("open should resolve typst.plate_file and compile");
    assert!(session.page_count() >= 1);
}

#[test]
fn missing_plate_file_errors_at_open_not_load() {
    // Loads fine: core reads no backend template at load time.
    let q = quill(YAML, &[]);
    // Opening resolves the plate and fails because the declared file is absent.
    let err = match TypstBackend.open(&q, &serde_json::json!({})) {
        Ok(_) => panic!("a missing plate file must fail at open"),
        Err(e) => e,
    };
    let diags = err.into_diagnostics();
    assert!(
        diags
            .iter()
            .any(|d| d.code.as_deref() == Some("typst::plate_missing")),
        "expected a typst::plate_missing diagnostic, got {:?}",
        diags.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>()
    );
}
