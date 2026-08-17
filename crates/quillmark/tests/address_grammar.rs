//! One schema address grammar, two implementations. pdfform's `bind` walks the
//! `QuillConfig`; the Typst helper's `_qm-known-path` reads the address tables
//! `SchemaMeta` derives from the transform schema. `PLATE_DATA.md` promises a
//! plate author that one address binds on either backend, and [`GRAMMAR`] is
//! what holds the two to it — neither side can move alone.
//!
//! The two content addresses sit outside that promise:
//! [`a_body_address_is_typst_only`] pins them so the asymmetry reads as declared
//! rather than as drift.

#![cfg(all(feature = "typst", feature = "pdfform"))]

use quillmark::{Backend, FileTreeNode, Quill};
use quillmark_typst::TypstBackend;
use std::collections::HashMap;

/// Every position the one-level nesting contract admits, declared once at card
/// level and once inside a card kind so each address has its card twin.
const YAML: &str = r#"
quill:
  name: address_grammar
  version: 0.1.0
  backend: typst
  description: every position the nesting contract admits
typst:
  plate_file: plate.typ
main:
  fields:
    subject:
      type: string
    tags:
      type: array
      items:
        type: string
    refs:
      type: array
      items:
        type: object
        properties:
          org: { type: string }
          num: { type: string }
    address:
      type: object
      properties:
        city: { type: string }
        street: { type: string }
    classification:
      type: enum
      values: [UNCLASSIFIED, CUI]
      default: ""
      variants:
        CUI:
          poc: { type: string }
card_kinds:
  endorsement:
    fields:
      from:
        type: string
      tags:
        type: array
        items:
          type: string
      refs:
        type: array
        items:
          type: object
          properties:
            org: { type: string }
            num: { type: string }
      origin:
        type: object
        properties:
          office: { type: string }
          city: { type: string }
      level:
        type: enum
        values: [FIRST, SECOND]
        default: ""
        variants:
          SECOND:
            endorser: { type: string }
"#;

/// Every address both backends must resolve, and every one both must refuse.
///
/// An index is not bounds-checked at address time on either side — `refs.9` and
/// the ninth endorsement resolve against a document carrying one of each — so
/// the grammar is a question about the schema alone.
const GRAMMAR: &[(&str, bool)] = &[
    ("subject", true),
    ("tags", true),
    ("tags.0", true),
    ("refs", true),
    ("refs.0", true),
    ("refs.9", true),
    ("refs.0.org", true),
    ("address", true),
    ("address.city", true),
    ("classification", true),
    ("classification.value", true),
    ("classification.poc", true),
    ("$cards.endorsement.0.from", true),
    ("$cards.endorsement.9.from", true),
    ("$cards.endorsement.0.tags", true),
    ("$cards.endorsement.0.tags.0", true),
    ("$cards.endorsement.0.refs", true),
    ("$cards.endorsement.0.refs.0", true),
    ("$cards.endorsement.0.refs.0.org", true),
    ("$cards.endorsement.0.origin", true),
    ("$cards.endorsement.0.origin.office", true),
    ("$cards.endorsement.0.level", true),
    ("$cards.endorsement.0.level.value", true),
    ("$cards.endorsement.0.level.endorser", true),
    ("nonesuch", false),
    ("subject.0", false),
    ("subject.poc", false),
    ("tags.0.org", false),
    ("refs.org", false),
    ("refs.0.undeclared", false),
    ("address.9", false),
    ("address.zip", false),
    ("address.city.0", false),
    ("classification.0", false),
    ("classification.undeclared", false),
    ("classification.value.oops", false),
    ("$cards", false),
    ("$cards.endorsement", false),
    ("$cards.endorsement.0", false),
    ("$cards.0.from", false),
    ("$cards.nosuch.0.from", false),
    ("$cards.endorsement.x.from", false),
    ("$cards.endorsement.0.nosuch", false),
    ("$cards.endorsement.0.from.0", false),
    ("$cards.endorsement.0.tags.0.org", false),
    ("$cards.endorsement.0.origin.office.0", false),
];

const PREAMBLE: &str = r#"#import "@local/quillmark-helper:0.1.0": field-region
#set page(width: 400pt, height: 1200pt, margin: 20pt)
"#;

fn quill(plate: &str) -> Quill {
    let mut files = HashMap::new();
    for (name, contents) in [("Quill.yaml", YAML), ("plate.typ", plate)] {
        files.insert(
            name.to_string(),
            FileTreeNode::File {
                contents: contents.as_bytes().to_vec(),
            },
        );
    }
    Quill::from_tree(FileTreeNode::Directory { files }).expect("load quill")
}

fn data() -> serde_json::Value {
    serde_json::json!({
        "subject": "Widgets",
        "tags": ["urgent"],
        "refs": [{ "org": "AFRL/RQ", "num": "2026-01" }],
        "address": { "city": "Dayton", "street": "1864 Fourth St" },
        "classification": { "value": "CUI", "poc": "Capt J. Smith" },
        "$cards": [{
            "$kind": "endorsement",
            "from": "SAF/AA",
            "tags": ["routine"],
            "refs": [{ "org": "AFRL/RQ", "num": "2026-02" }],
            "origin": { "office": "SAF/AA", "city": "Dayton" },
            "level": { "value": "SECOND", "endorser": "Col K. Lee" },
        }],
    })
}

/// A plate claiming each of `addresses`, so one compile answers a whole column.
fn claims(addresses: impl IntoIterator<Item = &'static str>) -> String {
    addresses
        .into_iter()
        .fold(PREAMBLE.to_string(), |mut plate, address| {
            plate.push_str(&format!("#field-region(\"{address}\")[x]\n"));
            plate
        })
}

fn compile(plate: &str) -> Result<quillmark::LiveSession, String> {
    TypstBackend
        .open(&quill(plate), &data())
        .map_err(|e| format!("{e}"))
}

#[test]
fn pdfform_resolves_exactly_the_shared_grammar() {
    let quill = quill("");
    for (address, resolves) in GRAMMAR {
        assert_eq!(
            quillmark_pdfform::resolves_schema_address(quill.config(), address),
            *resolves,
            "{address:?}"
        );
    }
}

#[test]
fn typst_admits_every_address_the_grammar_resolves() {
    let accepted = GRAMMAR.iter().filter(|(_, ok)| *ok).map(|(a, _)| *a);
    if let Err(err) = compile(&claims(accepted)) {
        panic!("every resolving address must compile: {err}");
    }
}

#[test]
fn typst_refuses_every_address_the_grammar_refuses() {
    for (address, _) in GRAMMAR.iter().filter(|(_, ok)| !*ok) {
        let Err(err) = compile(&claims([*address])) else {
            panic!("{address:?} must not compile");
        };
        assert!(err.contains("not a schema field address"), "{address:?}: {err}");
    }
}

/// A body is content rather than a widget-bindable field, so the promise
/// [`GRAMMAR`] pins stops at the two body addresses: a plate writes them and
/// pdfform's resolver roots neither.
#[test]
fn a_body_address_is_typst_only() {
    let bodies = ["$body", "$cards.endorsement.0.$body"];
    let quill = quill("");
    for address in bodies {
        assert!(
            !quillmark_pdfform::resolves_schema_address(quill.config(), address),
            "{address:?}"
        );
    }
    if let Err(err) = compile(&claims(bodies)) {
        panic!("a plate claims a body address: {err}");
    }
}
