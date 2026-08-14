//! `field-region` through the public `Backend`/`LiveSession` path: what a
//! preview consumer actually reads back.

use quillmark_core::Backend;
use quillmark_typst::TypstBackend;

mod common;

const YAML: &str = r#"
quill:
  name: field_region
  version: 0.1.0
  backend: typst
  description: field-region acceptance
typst:
  plate_file: plate.typ
main:
  fields:
    classification:
      type: string
      description: a scalar the plate presents as a banner
    subject:
      type: string
      description: an unrelated scalar
"#;

fn open(plate: &str) -> quillmark_core::LiveSession {
    let source = common::quill_with_plate(YAML, plate);
    TypstBackend
        .open(
            &source,
            &serde_json::json!({ "classification": "SECRET", "subject": "Widgets" }),
        )
        .expect("open")
}

#[test]
fn a_claim_surfaces_as_a_region_and_answers_field_at() {
    let plate = r#"
#import "@local/quillmark-helper:0.1.0": data, field-region
#set page(width: 400pt, height: 200pt, margin: 40pt)
#let banner(level) = box(stroke: 1pt, inset: 6pt)[#upper(level)]
#field-region("classification")[#banner(data.classification)]
"#;
    let session = open(plate);
    let region = session
        .regions()
        .into_iter()
        .find(|r| r.field == "classification")
        .expect("the claim surfaces in the sidecar");
    assert!(region.span.is_none(), "a claim carries no content span");

    let (cx, cy) = (
        (region.rect[0] + region.rect[2]) / 2.0,
        (region.rect[1] + region.rect[3]) / 2.0,
    );
    assert_eq!(
        session.field_at(region.page, cx, cy).as_deref(),
        Some("classification"),
        "a click inside the claim routes to its field"
    );
}

/// The banner's text is `data.classification` laundered through `upper`, so the
/// scalar site owns it and the claim owns the box the plate drew around it:
/// wrapping adds a region, it never moves one.
#[test]
fn a_claim_does_not_displace_a_nested_scalar_site() {
    let plate = r#"
#import "@local/quillmark-helper:0.1.0": data, field-region
#set page(width: 400pt, height: 200pt, margin: 40pt)
#field-region("subject")[Level: #data.classification]
"#;
    let regions = open(plate).regions();
    for field in ["subject", "classification"] {
        assert!(
            regions.iter().any(|r| r.field == field),
            "{field:?} keeps a region of its own: {regions:?}"
        );
    }
}

#[test]
fn an_unknown_field_address_fails_the_compile() {
    let plate = r#"
#import "@local/quillmark-helper:0.1.0": field-region
#field-region("not_a_field")[x]
"#;
    let source = common::quill_with_plate(YAML, plate);
    let Err(err) = TypstBackend.open(&source, &serde_json::json!({})) else {
        panic!("an unknown address must not compile");
    };
    assert!(
        format!("{err}").contains("not a schema field address"),
        "the assert names the problem: {err}"
    );
}
