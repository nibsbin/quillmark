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
//!
//! The typed read (`BINDINGS.md` § "Typed reader front door") and the preview
//! queries (`PREVIEW.md`) are the same claim on the other two flows:
//! `Quill::reader` returns a `TypedReader`, whose verbs yield `ReadValue` and
//! `CardReader`, and a `LiveSession` answers in `RenderedRegion` and
//! `ContentHit`. Each test annotates the return types explicitly — an inferred
//! binding would compile with the name absent and gate nothing.

use std::collections::HashMap;

use quillmark::{
    BoundParseError, CardReader, Document, EditError, FileTreeNode, Parsed, Quill, QuillReference,
    QuillValue, ReadValue, TypedReader, TypedWriter,
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
    subject:
      type: richtext
      inline: true

card_kinds:
  note:
    fields:
      body:
        type: richtext
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

#[test]
fn typed_read_spells_through_the_facade() {
    let quill = quill();
    let reference: QuillReference = "facade_surface".parse().expect("reference parses");
    let mut doc = Document::new(reference);
    {
        let mut writer: TypedWriter = quill.writer(&mut doc);
        writer.set("subject", "Hello **world**").expect("richtext");
        writer
            .add_card("note", [("body", "a *card*")], None, None)
            .expect("note is a declared card kind");
    }

    let reader: TypedReader = quill.reader(&doc);

    let subject: Option<ReadValue> = reader.get("subject").expect("subject is declared");
    assert!(
        matches!(subject, Some(ReadValue::Markdown(ref md)) if md == "Hello **world**"),
        "a richtext field projects to markdown: {subject:?}"
    );

    // The authority the quill-free projection lacked: an undeclared name is a
    // typo, not an absence.
    let typo: Result<Option<ReadValue>, EditError> = reader.get("nope");
    assert!(matches!(typo, Err(EditError::UnknownField(_))), "{typo:?}");

    let card: CardReader = reader.card(0).expect("the note card resolves its schema");
    assert_eq!(card.kind(), Some("note"));
    let body: Option<ReadValue> = card.get("body").expect("body is declared on `note`");
    assert!(matches!(body, Some(ReadValue::Markdown(ref md)) if md == "a *card*"), "{body:?}");
}

/// The preview flow needs a real backend session, so it rides the typst
/// feature. `RenderedRegion` and `ContentHit` are what every query here
/// answers in; `HitGranularity` rides inside the hit.
#[cfg(feature = "typst")]
#[test]
fn preview_regions_spell_through_the_facade() {
    use quillmark::{ContentHit, HitGranularity, LiveSession, Quillmark, RenderedRegion};

    let engine = Quillmark::new();
    let quill = quillmark::quill_from_path(quillmark_fixtures::quills_path("usaf_memo"))
        .expect("usaf_memo should load");
    let parsed = quill.seed_document();
    let session: LiveSession = engine.open(&quill, &parsed).expect("open a session");

    let regions: Vec<RenderedRegion> = session.regions();
    // Span-bearing: `field_boxes` and `position_at` are content-only, so a
    // scalar-reference or widget region would answer nothing through them.
    let region: &RenderedRegion = regions
        .iter()
        .find(|r| r.span.is_some())
        .expect("the seeded memo places at least one content field");

    let boxes: Vec<RenderedRegion> = session.field_boxes(&region.field);
    assert!(!boxes.is_empty(), "a content field unions to at least one box");

    let cx = (region.rect[0] + region.rect[2]) / 2.0;
    let cy = (region.rect[1] + region.rect[3]) / 2.0;
    assert_eq!(session.field_at(region.page, cx, cy).as_deref(), Some(region.field.as_str()));

    let hit: Option<ContentHit> = session.position_at(region.page, cx, cy);
    let hit: ContentHit = hit.expect("a content region's centre hit-tests to a content position");
    assert_eq!(hit.field, region.field);
    let _granularity: Option<HitGranularity> = hit.granularity;

    let located: Option<RenderedRegion> = session.locate(&region.field, 0);
    assert!(located.is_some(), "a field with a region locates position 0");
}

/// The engine, a loaded quill, and a document cross threads, so a server can
/// hold one `Quillmark` and render on many.
///
/// `docs/integration/operations.md` § "Concurrency" states this; the assertions
/// are what stop a private field quietly taking it away.
#[test]
fn engine_and_inputs_are_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<quillmark::Quillmark>();
    assert_send_sync::<Quill>();
    assert_send_sync::<Document>();
}
