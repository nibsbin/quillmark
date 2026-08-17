//! The container property step (`classification.poc`, `address.city`) through
//! the public `Backend`/`LiveSession` path: what the address grammar accepts,
//! what it still rejects, and the address a region carries for a plate that
//! reads one property.

use quillmark_core::Backend;
use quillmark_typst::TypstBackend;

mod common;

const YAML: &str = r#"
quill:
  name: container_address
  version: 0.1.0
  backend: typst
  description: container property addressing
typst:
  plate_file: plate.typ
main:
  fields:
    subject:
      type: string
      description: a scalar, which offers no step at all
    address:
      type: object
      description: a typed dictionary
      properties:
        city:
          type: string
        street:
          type: string
    classification:
      type: enum
      values: [UNCLASSIFIED, CUI]
      default: ""
      description: a variant container
      variants:
        CUI:
          poc:
            type: string
          controlled_by:
            type: string
"#;

fn data() -> serde_json::Value {
    serde_json::json!({
        "subject": "Widgets",
        "address": { "city": "Dayton", "street": "1864 Fourth St" },
        "classification": { "value": "CUI", "poc": "Capt J. Smith", "controlled_by": "SAF/AA" },
    })
}

fn open(plate: &str) -> quillmark_core::LiveSession {
    TypstBackend
        .open(&common::quill_with_plate(YAML, plate), &data())
        .expect("open")
}

fn rejects(plate: &str) -> String {
    let source = common::quill_with_plate(YAML, plate);
    let Err(err) = TypstBackend.open(&source, &data()) else {
        panic!("the address must not compile");
    };
    format!("{err}")
}

/// A property read anchors on the property, so the region names the cell the
/// plate read rather than the container holding it.
#[test]
fn a_property_read_regions_on_the_property() {
    let plate = r#"
#import "@local/quillmark-helper:0.1.0": data
#set page(width: 400pt, height: 200pt, margin: 40pt)
#data.classification.poc
#data.address.city
"#;
    let session = open(plate);
    let regions = session.regions();
    for field in ["classification.poc", "address.city"] {
        assert!(
            regions.iter().any(|r| r.field == field),
            "{field:?} regions on its own address: {regions:?}"
        );
    }
    assert!(
        !regions.iter().any(|r| r.field == "classification"),
        "the container does not also claim the property's ink: {regions:?}"
    );

    let poc = regions
        .iter()
        .find(|r| r.field == "classification.poc")
        .expect("the property region surfaces");
    let (cx, cy) = (
        (poc.rect[0] + poc.rect[2]) / 2.0,
        (poc.rect[1] + poc.rect[3]) / 2.0,
    );
    assert_eq!(
        session.field_at(poc.page, cx, cy).as_deref(),
        Some("classification.poc"),
        "a click on the cell routes to the cell, not the container"
    );
}

/// A bound read surfaces the region the direct chain surfaces, so naming a
/// container before stepping into it costs no address.
#[test]
fn a_property_read_through_a_let_alias_keeps_the_property_address() {
    let plate = r#"
#import "@local/quillmark-helper:0.1.0": data
#set page(width: 400pt, height: 200pt, margin: 40pt)
#let c = data.classification
#let a = data.at("address", default: (:))
#c.poc #c.at("controlled_by") #a.city
"#;
    let session = open(plate);
    let regions = session.regions();
    for field in ["classification.poc", "classification.controlled_by", "address.city"] {
        assert!(
            regions.iter().any(|r| r.field == field),
            "{field:?} regions through the alias: {regions:?}"
        );
    }

    let poc = regions
        .iter()
        .find(|r| r.field == "classification.poc")
        .expect("the property region surfaces");
    let (cx, cy) = (
        (poc.rect[0] + poc.rect[2]) / 2.0,
        (poc.rect[1] + poc.rect[3]) / 2.0,
    );
    assert_eq!(
        session.field_at(poc.page, cx, cy).as_deref(),
        Some("classification.poc"),
        "a click on a bound read routes to the cell it read"
    );
}

/// A name the plate could rebind is not followed, so a stale alias never
/// attributes another value's ink to the field.
#[test]
fn a_rebound_alias_surfaces_no_region() {
    let plate = r#"
#import "@local/quillmark-helper:0.1.0": data
#set page(width: 400pt, height: 200pt, margin: 40pt)
#let c = data.classification
#let c = (poc: "someone else")
#c.poc
"#;
    let regions = open(plate).regions();
    assert!(
        !regions.iter().any(|r| r.field.starts_with("classification")),
        "a rebound name carries no address: {regions:?}"
    );
}

/// The container read whole is still one region: the step is what narrows it.
#[test]
fn the_container_read_whole_still_regions_on_the_container() {
    let plate = r#"
#import "@local/quillmark-helper:0.1.0": data
#set page(width: 400pt, height: 200pt, margin: 40pt)
#data.classification.value
#repr(data.address)
"#;
    let regions = open(plate).regions();
    for field in ["classification.value", "address"] {
        assert!(
            regions.iter().any(|r| r.field == field),
            "{field:?} regions: {regions:?}"
        );
    }
}

/// The whole point of the tables: a widget and a claim can now name a cell.
#[test]
fn a_widget_and_a_claim_bind_a_container_property() {
    let plate = r#"
#import "@local/quillmark-helper:0.1.0": data, field-region, form-field
#set page(width: 400pt, height: 200pt, margin: 40pt)
#form-field("Poc", type: "text", value: data.classification.poc, field: "classification.poc")
#field-region("address.city")[#box(stroke: 1pt, inset: 4pt)[#upper(data.address.city)]]
"#;
    let regions = open(plate).regions();
    for field in ["classification.poc", "address.city"] {
        assert!(
            regions.iter().any(|r| r.field == field),
            "{field:?} surfaces: {regions:?}"
        );
    }
}

/// A card's container fields ride the same table, keyed through the card's
/// `$path` prefix.
#[test]
fn a_card_container_property_is_addressable() {
    const CARD_YAML: &str = r#"
quill:
  name: card_container_address
  version: 0.1.0
  backend: typst
  description: card container addressing
typst:
  plate_file: plate.typ
main:
  body:
    enabled: false
card_kinds:
  endorsement:
    fields:
      origin:
        type: object
        properties:
          office:
            type: string
"#;
    let plate = r#"
#import "@local/quillmark-helper:0.1.0": data, field-region
#set page(width: 400pt, height: 200pt, margin: 40pt)
#for card in data.at("$cards") {
  field-region(card.at("$path") + "origin.office")[#card.origin.office]
}
"#;
    let session = TypstBackend
        .open(
            &common::quill_with_plate(CARD_YAML, plate),
            &serde_json::json!({
                "$cards": [ { "$kind": "endorsement", "origin": { "office": "SAF/AA" } } ]
            }),
        )
        .expect("open");
    let regions = session.regions();
    assert!(
        regions
            .iter()
            .any(|r| r.field == "$cards.endorsement.0.origin.office"),
        "the card's container property claims its ink: {regions:?}"
    );
}

/// The step is gated on what the field offers, so a scalar admits neither an
/// index nor a property, and a container admits only its declared keys.
#[test]
fn the_step_is_gated_on_the_declared_shape() {
    for (address, plate_field) in [
        ("subject.poc", "a scalar has no property"),
        ("classification.undeclared", "an undeclared key is no cell"),
        ("classification.0", "a container has no element"),
        ("address.9", "a typed dictionary has no element"),
    ] {
        let plate = format!(
            r#"
#import "@local/quillmark-helper:0.1.0": field-region
#field-region("{address}")[x]
"#
        );
        let err = rejects(&plate);
        assert!(
            err.contains("not a schema field address"),
            "{plate_field} ({address:?}): {err}"
        );
    }
}
