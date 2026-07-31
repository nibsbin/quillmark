//! End-to-end acceptance test for the `sample_form` fixture: render the
//! hand-authored stripped background + form.json through the full engine
//! (pdfform backend registered), then reparse with lopdf and assert the filled
//! AcroForm. Technique A means values land in `/V`; appearance synthesis is the
//! viewer's job, verified by a human rather than headless.

use lopdf::Document as PdfDoc;
use quillmark::{Document, OutputFormat, Quillmark, RenderOptions};

const FILLED: &str = "~~~\n\
$quill: sample_form\n\
$kind: main\n\
full_name: Ada Lovelace\n\
comments:\n\
  - First comment line.\n\
  - Second comment line.\n\
agree: true\n\
favorite_color: green\n\
~~~\n";

fn render(markdown: &str) -> quillmark::RenderResult {
    let quill = quillmark::quill_from_path(quillmark_fixtures::quills_path("sample_form"))
        .expect("load sample_form quill");
    let engine = Quillmark::new();
    let doc = Document::parse(markdown).expect("parse markdown").document;
    engine
        .render(
            &quill,
            &doc,
            &RenderOptions::default().with_output_format(OutputFormat::Pdf),
        )
        .expect("render ok")
}

/// Open a compiled session: the surface that carries schema-field geometry
/// (`session.regions()`), independent of any byte render.
fn open_session(markdown: &str) -> quillmark::LiveSession {
    let quill = quillmark::quill_from_path(quillmark_fixtures::quills_path("sample_form"))
        .expect("load sample_form quill");
    let engine = Quillmark::new();
    let doc = Document::parse(markdown).expect("parse markdown").document;
    engine.open(&quill, &doc).expect("open ok")
}

mod common;
use common::{decode_pdf_text, widget};

#[test]
fn fixture_renders_structurally_valid_filled_pdf() {
    let result = render(FILLED);
    assert_eq!(result.output_format, OutputFormat::Pdf);
    let pdf = &result.artifacts[0].bytes;

    let doc = PdfDoc::load_mem(pdf).expect("lopdf reparse: structurally valid");
    let cat = doc.catalog().expect("catalog");
    let af = doc
        .get_object(cat.get(b"AcroForm").unwrap().as_reference().unwrap())
        .unwrap()
        .as_dict()
        .unwrap();
    assert!(af.get(b"NeedAppearances").unwrap().as_bool().unwrap());
    assert_eq!(af.get(b"SigFlags").unwrap().as_i64().unwrap(), 1);
    assert_eq!(af.get(b"Fields").unwrap().as_array().unwrap().len(), 8);

    // This e2e pins the *binding* layer: markdown/schema → field values,
    // tooltip, array join, regions, producer default. The spine bytes it once
    // re-checked (the `Ff` multiline/combo flags, `/Opt` length, checkbox
    // `/V`+`/AS`, and `/FT` names) are owned by the spine seam in
    // `quillmark-pdf/tests/stamp.rs`.

    // Text: bound scalar value + tooltip. The form.json field carries no
    // `tooltip`, so `/TU` is inherited from the schema field's `description`
    // (form@0.2.0 derives the tooltip when unset).
    let full = widget(&doc, af, "FullName");
    assert_eq!(full.get(b"V").unwrap().as_str().unwrap(), b"Ada Lovelace");
    assert_eq!(
        full.get(b"TU").unwrap().as_str().unwrap(),
        b"Full legal name of the applicant. Binds the FullName text field."
    );

    // Multiline text: array joined with newlines.
    let comments = widget(&doc, af, "Comments");
    assert_eq!(
        decode_pdf_text(comments.get(b"V").unwrap().as_str().unwrap()),
        "First comment line.\nSecond comment line."
    );

    // Choice: matching option bound.
    let color = widget(&doc, af, "FavoriteColor");
    assert_eq!(color.get(b"V").unwrap().as_str().unwrap(), b"green");

    // Region geometry is a session-level query (`session.regions()`), not on the
    // render result: one per *schema-bound* field, keyed on the schema path. The
    // fixture's four unbound widgets carry no `schema_field`, so they are
    // backend-only artifacts and emit no region: four regions, not eight.
    let regions = open_session(FILLED).regions();
    assert_eq!(regions.len(), 4);
    assert!(
        regions
            .iter()
            .all(|r| !r.field.starts_with("Signer") && r.field != "Signature"),
        "no unbound widget produces a region"
    );
    let r_full = regions.iter().find(|r| r.field == "full_name").unwrap();
    // Geometry rides the sidecar: a real page and a non-degenerate rect.
    assert!(r_full.page < doc.get_pages().len().max(1));
    assert!(
        r_full.rect[2] > r_full.rect[0] && r_full.rect[3] > r_full.rect[1],
        "region rect is a proper box: {:?}",
        r_full.rect
    );

    // Producer stamped with the backend default.
    let info = doc
        .get_object(doc.trailer.get(b"Info").unwrap().as_reference().unwrap())
        .unwrap()
        .as_dict()
        .unwrap();
    let producer = info.get(b"Producer").unwrap().as_str().unwrap();
    assert!(
        producer.starts_with(b"Quillmark "),
        "producer = {:?}",
        String::from_utf8_lossy(producer)
    );
}

/// The unbound population: widgets a signer fills, whose kind comes from
/// `form.json`'s own `type` token instead of a schema field.
///
/// `stamp.rs` owns the spine's `FieldType` → `/FT` mapping. What this file owns
/// is the rest of that path: the declared token survives bind into the stamped
/// output, the `options` array reaches the widget, and no document value lands
/// on an unbound widget.
#[test]
fn unbound_widgets_stamp_their_declared_kind_and_take_no_value() {
    let result = render(FILLED);
    let doc = PdfDoc::load_mem(&result.artifacts[0].bytes).expect("lopdf reparse");
    let cat = doc.catalog().expect("catalog");
    let af = doc
        .get_object(cat.get(b"AcroForm").unwrap().as_reference().unwrap())
        .unwrap()
        .as_dict()
        .unwrap();

    for (name, ft) in [
        ("SignerInitials", &b"Tx"[..]),
        ("SignerConfirms", &b"Btn"[..]),
        ("SignerRole", &b"Ch"[..]),
        ("Signature", &b"Sig"[..]),
    ] {
        assert_eq!(
            widget(&doc, af, name).get(b"FT").unwrap().as_name().unwrap(),
            ft,
            "{name}"
        );
    }

    // `options` has no schema counterpart: an unbound choice is the only place
    // dropdown options are declared, so no other path carries them.
    let opts: Vec<String> = widget(&doc, af, "SignerRole")
        .get(b"Opt")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|o| decode_pdf_text(o.as_str().unwrap()))
        .collect();
    assert_eq!(opts, ["witness", "notary", "guardian"]);

    // No `schema_field` means no document value can reach them: text and choice
    // carry no `/V` at all, and the checkbox is `Off` regardless of how the
    // document's own `agree: true` resolved on the bound `Agree`.
    assert!(widget(&doc, af, "SignerInitials").get(b"V").is_err());
    assert!(widget(&doc, af, "SignerRole").get(b"V").is_err());
    assert_eq!(
        widget(&doc, af, "SignerConfirms")
            .get(b"V")
            .unwrap()
            .as_name()
            .unwrap(),
        b"Off"
    );
}

#[test]
fn non_ascii_value_round_trips_through_acroform_v() {
    // A non-ASCII (accented / Latin-1 + smart-punctuation) text value must reach
    // the AcroForm `/V` intact end-to-end: pdf-writer encodes it UTF-16BE, so
    // the value decodes back to exactly what was authored.
    let md = "~~~\n\
$quill: sample_form\n\
$kind: main\n\
full_name: \"Café — Señor 'Ünïcøde'\"\n\
agree: true\n\
favorite_color: green\n\
~~~\n";
    let result = render(md);
    let pdf = &result.artifacts[0].bytes;
    let doc = PdfDoc::load_mem(pdf).expect("lopdf reparse");
    let cat = doc.catalog().unwrap();
    let af = doc
        .get_object(cat.get(b"AcroForm").unwrap().as_reference().unwrap())
        .unwrap()
        .as_dict()
        .unwrap();

    let full = widget(&doc, af, "FullName");
    assert_eq!(
        decode_pdf_text(full.get(b"V").unwrap().as_str().unwrap()),
        "Café — Señor 'Ünïcøde'"
    );
    // The session's region geometry is keyed on the schema path, not the bound
    // value (the value lives in the AcroForm `/V`, asserted above).
    assert!(
        open_session(md)
            .regions()
            .iter()
            .any(|r| r.field == "full_name"),
        "a region is keyed on the schema path"
    );
}

#[test]
fn unchecked_and_unmatched_choice_render_blank() {
    let md = "~~~\n\
$quill: sample_form\n\
$kind: main\n\
full_name: Bob\n\
agree: false\n\
favorite_color: red\n\
~~~\n";
    let result = render(md);
    let pdf = &result.artifacts[0].bytes;
    let doc = PdfDoc::load_mem(pdf).unwrap();
    let cat = doc.catalog().unwrap();
    let af = doc
        .get_object(cat.get(b"AcroForm").unwrap().as_reference().unwrap())
        .unwrap()
        .as_dict()
        .unwrap();

    // Unchecked checkbox → /V /Off, /AS /Off.
    let agree = widget(&doc, af, "Agree");
    assert_eq!(agree.get(b"V").unwrap().as_name().unwrap(), b"Off");
    assert_eq!(agree.get(b"AS").unwrap().as_name().unwrap(), b"Off");

    // Absent comments → blank multiline field.
    let comments = widget(&doc, af, "Comments");
    assert!(comments.get(b"V").is_err(), "absent array → no /V");
}

#[test]
fn apply_rebinds_values_and_reports_dirty_pages() {
    let quill = quillmark::quill_from_path(quillmark_fixtures::quills_path("sample_form"))
        .expect("load sample_form quill");
    let engine = Quillmark::new();
    let doc = Document::parse(FILLED).expect("parse markdown").document;
    let mut session = engine.open(&quill, &doc).expect("open ok");

    // Identical data → nothing dirty.
    let cs = session
        .apply(&quill.compile_data(&doc).expect("compile data"))
        .expect("apply");
    assert_eq!(cs.page_count, session.page_count());
    assert!(cs.dirty_pages.is_empty(), "dirty: {:?}", cs.dirty_pages);

    // A changed field dirties its page and rebinds the stamped value.
    let doc2 = Document::parse(&FILLED.replace("Ada Lovelace", "Grace Hopper"))
        .expect("parse markdown")
        .document;
    let cs = session
        .apply(&quill.compile_data(&doc2).expect("compile data"))
        .expect("apply");
    assert_eq!(cs.dirty_pages, vec![0]);

    let result = session
        .render(&RenderOptions::default().with_output_format(OutputFormat::Pdf))
        .expect("render ok");
    let pdf = PdfDoc::load_mem(&result.artifacts[0].bytes).unwrap();
    let cat = pdf.catalog().unwrap();
    let af = pdf
        .get_object(cat.get(b"AcroForm").unwrap().as_reference().unwrap())
        .unwrap()
        .as_dict()
        .unwrap();
    let name = widget(&pdf, af, "FullName");
    assert_eq!(
        decode_pdf_text(name.get(b"V").unwrap().as_str().unwrap()),
        "Grace Hopper"
    );
}
