//! The facade spells every documented flow on its own. This file names only
//! `quillmark::`, so a type dropping out of the re-export list in
//! `crates/quillmark/src/lib.rs` stops it compiling.
//!
//! Authoring (`PROGRAMMATIC.md`) and bound-door ingestion (`Quill::parse`,
//! `prose/canon/` via `quill/conform.rs`) are the flows a render-only re-export
//! set leaves unspellable: `Document::new` takes a `QuillReference`,
//! `Quill::writer` returns a `TypedWriter`, the writer verbs fail as
//! `EditError`, `Quill::parse` fails as `BoundParseError`, and a field reads
//! back as a `QuillValue`.

use std::collections::HashMap;

use quillmark::{
    BoundParseError, Document, EditError, FileTreeNode, Parsed, Quill, QuillReference, QuillValue,
    TypedWriter,
};

const QUILL: &str = r#"
quill:
  name: facade_surface
  version: "1.0"
  backend: typst
  description: Facade re-export surface test

main:
  fields:
    title:
      type: string
"#;

fn quill() -> Quill {
    let mut files = HashMap::new();
    files.insert(
        "Quill.yaml".to_string(),
        FileTreeNode::File {
            contents: QUILL.as_bytes().to_vec(),
        },
    );
    Quill::from_tree(FileTreeNode::Directory { files }).expect("from_tree")
}

#[test]
fn authoring_spells_through_the_facade() {
    let quill = quill();
    let reference: QuillReference = "facade_surface".parse().expect("reference parses");
    let mut doc = Document::new(reference);

    let mut writer: TypedWriter = quill.writer(&mut doc);
    let written: Result<(), EditError> = writer.set("title", "Hello");
    written.expect("title is a declared string field");

    let title: Option<&QuillValue> = doc.main().payload().get("title");
    assert_eq!(title.and_then(|v| v.as_str()), Some("Hello"));
}

#[test]
fn bound_parse_spells_through_the_facade() {
    let quill = quill();
    let md = "~~~\n$quill: facade_surface\n$kind: main\ntitle: Hello\n~~~\n\n# Body\n";

    let parsed: Result<Parsed, BoundParseError> = quill.parse(md);
    let parsed = parsed.expect("document matches the quill");
    assert_eq!(
        parsed
            .document
            .main()
            .payload()
            .get("title")
            .and_then(|v| v.as_str()),
        Some("Hello")
    );

    let elsewhere = md.replace("facade_surface", "other_quill");
    let mismatch: Result<Parsed, BoundParseError> = quill.parse(&elsewhere);
    assert!(
        mismatch.is_err(),
        "a $quill naming another quill fails at the bound door"
    );
}
