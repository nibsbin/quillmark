//! The value axis's other half: a cell holding the value its schema showed.

use crate::quill::quill_from_yaml;
use crate::{Document, Quill};

fn unchanged(quill: &Quill, md: &str) -> Vec<(String, String)> {
    let doc = Document::parse(md).expect("parse").document;
    let mut out: Vec<(String, String)> = quill
        .validate(&doc)
        .iter()
        .filter(|d| d.code.as_deref() == Some("validation::example_unchanged"))
        .map(|d| {
            (
                d.path.clone().expect("every example_unchanged anchors at a path"),
                d.args["trigger"].as_str().expect("trigger arg").to_string(),
            )
        })
        .collect();
    out.sort();
    out
}

fn paths(quill: &Quill, md: &str) -> Vec<String> {
    unchanged(quill, md).into_iter().map(|(p, _)| p).collect()
}

const MEMO: &str = r#"
quill: { name: eg, version: 1.0.0, backend: typst, description: examples }
main:
  body:
    example: The first paragraph. Top-level paragraphs are auto-numbered.
  fields:
    subject:  { type: string, example: Duty Title }
    memo_for: { type: array, items: { type: string }, example: [ORG1/SYMBOL, ORG2/SYMBOL] }
    freeform: { type: string }
    status:   { type: string, default: draft, example: final }
    signer:
      type: object
      properties:
        name:  { type: string, example: FIRST M. LAST }
        grade: { type: string, example: "Rank, USAF" }
card_kinds:
  indorsement:
    fields:
      from: { type: string, example: FIRST M. LAST }
"#;

fn md(fields: &str) -> String {
    format!("~~~card-yaml\n$quill: eg@1.0.0\n$kind: main\n{fields}~~~\n")
}

fn with_body(fields: &str, body: &str) -> String {
    format!("{}\n{body}\n", md(fields))
}

#[test]
fn a_cell_left_at_its_example_warns_and_an_authored_one_does_not() {
    let quill = quill_from_yaml(MEMO);

    assert_eq!(
        unchanged(&quill, &md("subject: Duty Title\n")),
        [("main.subject".to_string(), "field".to_string())]
    );
    assert!(paths(&quill, &md("subject: Airfield closure\n")).is_empty());
}

#[test]
fn a_field_declaring_no_example_never_warns() {
    let quill = quill_from_yaml(MEMO);

    for value in ["freeform: Duty Title\n", "freeform: \"\"\n", "freeform: x\n"] {
        assert!(
            paths(&quill, &md(value)).is_empty(),
            "nothing to recognize: {value}"
        );
    }
}

/// The seed commits an example whether or not the field is obliged, so the
/// warning is keyed on the value, not on the obligation axis.
#[test]
fn a_defaulted_fields_example_warns_too() {
    let quill = quill_from_yaml(MEMO);

    assert_eq!(paths(&quill, &md("status: final\n")), ["main.status"]);
    assert!(paths(&quill, &md("status: signed\n")).is_empty());
}

#[test]
fn an_array_warns_at_the_element_left_behind() {
    let quill = quill_from_yaml(MEMO);

    assert_eq!(
        paths(&quill, &md("memo_for: [AF/A1, ORG2/SYMBOL]\n")),
        ["main.memo_for[1]"],
        "a partially edited array warns at the untouched element, not the container"
    );
    assert_eq!(
        paths(&quill, &md("memo_for: [ORG1/SYMBOL, ORG2/SYMBOL]\n")),
        ["main.memo_for[0]", "main.memo_for[1]"]
    );
    assert!(paths(&quill, &md("memo_for: [AF/A1, AF/A2]\n")).is_empty());
}

/// Position is part of the value: an example element the author moved is one
/// they handled.
#[test]
fn an_array_compares_index_for_index() {
    let quill = quill_from_yaml(MEMO);

    assert!(paths(&quill, &md("memo_for: [ORG2/SYMBOL, AF/A1]\n")).is_empty());
}

#[test]
fn a_typed_dictionary_warns_at_the_property_that_holds_the_example() {
    let quill = quill_from_yaml(MEMO);

    assert_eq!(
        paths(
            &quill,
            &md("signer:\n  name: FIRST M. LAST\n  grade: Col, USAF\n")
        ),
        ["main.signer.name"]
    );
}

#[test]
fn a_composable_card_is_judged_per_instance() {
    let quill = quill_from_yaml(MEMO);
    let root = md("subject: Airfield closure\n");
    let card = "\n~~~card-yaml\n$kind: indorsement\nfrom: FIRST M. LAST\n~~~\nReviewed.\n";

    assert_eq!(
        paths(&quill, &format!("{root}{card}{card}")),
        [
            "cards.indorsement[0].from",
            "cards.indorsement[1].from"
        ]
    );
}

#[test]
fn a_body_left_at_the_schemas_example_warns() {
    let quill = quill_from_yaml(MEMO);

    assert_eq!(
        unchanged(
            &quill,
            &with_body(
                "subject: Airfield closure\n",
                "The first paragraph. Top-level paragraphs are auto-numbered."
            )
        ),
        [("main.body".to_string(), "body".to_string())]
    );
    assert!(paths(
        &quill,
        &with_body("subject: Airfield closure\n", "The runway closes Friday.")
    )
    .is_empty());
}

#[test]
fn a_body_left_at_the_generated_placeholder_warns() {
    let quill = quill_from_yaml(MEMO);
    let root = md("subject: Airfield closure\n");

    let placeholder =
        "\n~~~card-yaml\n$kind: indorsement\nfrom: Ada\n~~~\nWrite indorsement body here.\n";
    assert_eq!(
        paths(&quill, &format!("{root}{placeholder}")),
        ["cards.indorsement[0].body"],
        "a kind declaring no `body.example` still shows text, and it is recognizable"
    );

    let written = "\n~~~card-yaml\n$kind: indorsement\nfrom: Ada\n~~~\nConcur.\n";
    assert!(paths(&quill, &format!("{root}{written}")).is_empty());
}

#[test]
fn an_empty_body_is_not_an_example() {
    let quill = quill_from_yaml(MEMO);

    assert!(paths(&quill, &md("subject: Airfield closure\n")).is_empty());
}

/// The blueprint stamps a body it emitted through the content model, so the
/// comparison meets it there rather than byte-for-byte.
#[test]
fn a_body_that_only_round_tripped_still_counts_as_untouched() {
    let quill = quill_from_yaml(
        r#"
quill: { name: rt, version: 1.0.0, backend: typst, description: round trip }
main:
  body:
    example: Write __this__ over.
  fields:
    subject: { type: string }
"#,
    );
    let md = "~~~card-yaml\n$quill: rt@1.0.0\n$kind: main\nsubject: S\n~~~\n\nWrite **this** over.\n";

    assert_eq!(paths(&quill, md), ["main.body"]);
}

const VARIANT: &str = r#"
quill: { name: vr, version: 1.0.0, backend: typst, description: variants }
main:
  fields:
    classification:
      type: enum
      values: [UNCLASSIFIED, CUI]
      example: CUI
      variants:
        CUI:
          controlled_by: { type: string, example: ORG1/SYMBOL }
          category:      { type: string }
"#;

fn vr(fields: &str) -> String {
    format!("~~~card-yaml\n$quill: vr@1.0.0\n$kind: main\n{fields}~~~\n")
}

#[test]
fn the_walk_reaches_the_selected_worlds_cells() {
    let quill = quill_from_yaml(VARIANT);

    assert_eq!(
        paths(
            &quill,
            &vr("classification:\n  value: CUI\n  controlled_by: ORG1/SYMBOL\n  category: PRVCY\n")
        ),
        ["main.classification.controlled_by", "main.classification.value"],
        "the discriminant and its live world are both cells the blueprint stamped"
    );

    assert!(
        paths(
            &quill,
            &vr("classification:\n  value: UNCLASSIFIED\n  controlled_by: ORG1/SYMBOL\n")
        )
        .is_empty(),
        "a stranded value belongs to `out_of_variant`, not to this walk"
    );
}
