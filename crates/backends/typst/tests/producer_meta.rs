//! Compiles plates to PDF, reparses with lopdf, and asserts the `/Info`
//! `/Producer` stamp.

use lopdf::Object;
use quillmark_core::{Backend, OutputFormat, RenderOptions};
use quillmark_typst::TypstBackend;

mod common;
use common::host_with_plate as source_with_plate;

const PLATE: &str = "#set page(width: 400pt, height: 300pt)\n= Hello\n";

fn render_pdf(plate: &str) -> Vec<u8> {
    let source = source_with_plate(plate);
    let session = TypstBackend
        .open(&source, &serde_json::json!({}))
        .expect("open session");
    let result = session
        .render(&RenderOptions::default().with_output_format(OutputFormat::Pdf))
        .expect("render ok");
    result.artifacts[0].bytes.clone()
}

fn producer_of(pdf: &[u8]) -> Vec<u8> {
    info_string(pdf, b"Producer")
}

fn info_string(pdf: &[u8], key: &[u8]) -> Vec<u8> {
    let doc = lopdf::Document::load_mem(pdf).expect("reparse pdf");
    let info_ref = doc
        .trailer
        .get(b"Info")
        .expect("/Info in trailer")
        .as_reference()
        .expect("/Info is a reference");
    let info = doc.get_object(info_ref).unwrap().as_dict().unwrap();
    match info.get(key).expect("key present in /Info") {
        Object::String(bytes, _) => bytes.clone(),
        other => panic!("{key:?} not a string: {other:?}"),
    }
}

#[test]
fn default_producer_is_quillmark_version() {
    let pdf = render_pdf(PLATE);
    let expected = format!("Quillmark {}", env!("CARGO_PKG_VERSION"));
    assert_eq!(producer_of(&pdf), expected.as_bytes());
}

#[test]
fn default_pass_preserves_typst_creator() {
    let pdf = render_pdf(PLATE);
    let creator = info_string(&pdf, b"Creator");
    assert!(
        creator.starts_with(b"Typst"),
        "expected Typst /Creator, got {:?}",
        String::from_utf8_lossy(&creator)
    );
}

#[test]
fn producer_override_via_render_options() {
    let source = source_with_plate(PLATE);
    let backend = TypstBackend;
    let session = backend
        .open(&source, &serde_json::json!({}))
        .expect("open session");
    // `()` and `\` exercise PDF literal-string escaping.
    let override_str = r"ACME (PDF) \ Tool 2.0";
    let result = session
        .render(&RenderOptions::default().with_output_format(OutputFormat::Pdf).with_producer(override_str.to_string()))
        .expect("render ok");
    let pdf = &result.artifacts[0].bytes;
    assert_eq!(producer_of(pdf), override_str.as_bytes());
}

#[test]
fn producer_composes_with_signature_field() {
    // One incremental update must carry both the widget and the stamp.
    let plate = r#"
#import "@local/quillmark-helper:0.1.0": signature-field
#set page(width: 600pt, height: 400pt, margin: 50pt)
#signature-field("a")
"#;
    let pdf = render_pdf(plate);

    let expected = format!("Quillmark {}", env!("CARGO_PKG_VERSION"));
    assert_eq!(producer_of(&pdf), expected.as_bytes());

    let doc = lopdf::Document::load_mem(&pdf).expect("reparse");
    let cat = doc.catalog().expect("catalog");
    let af_ref = cat.get(b"AcroForm").unwrap().as_reference().unwrap();
    let af = doc.get_object(af_ref).unwrap().as_dict().unwrap();
    let fields = af.get(b"Fields").unwrap().as_array().unwrap();
    assert_eq!(fields.len(), 1, "expected one signature field");
}
