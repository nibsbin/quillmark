//! A card block with no `$kind:` line is a *kindless* card. `to_plate_json`
//! omits `$kind` for one rather than fabricating `""`, so a plate that reads the
//! discriminator with a bare `card.at("$kind")` would panic the Typst compile on
//! it. It never gets the chance: every route from a document to plate data
//! (`compile_data`, and `compile_checked` beneath `Quillmark::render`) passes
//! `coerce_and_validate` first, and a card whose kind resolves to no schema is
//! fatal there.
//!
//! This pins that gate on the render path, for each fixture quill whose plate
//! dispatches on `$kind`: the failure is a path-anchored `validation::unknown_card`
//! naming the offending card, never a backend panic. Softening the gate to let a
//! kindless card through is what would make the plates' total `$kind` reads load-
//! bearing rather than merely canon-conforming.

#![cfg(feature = "typst")]

use quillmark::{Document, OutputFormat, Quillmark, RenderOptions};
use quillmark_fixtures::quills_path;

/// The seeded document (one card per declared kind) with a kindless card
/// appended, so the document is well-formed apart from the card under test.
fn with_kindless_card(quill: &quillmark::Quill) -> Document {
    let seeded = quill.seed_document().to_markdown();
    let markdown = format!("{seeded}\n~~~\nnote: kindless\n~~~\n\nBody of a kindless card.\n");
    let parsed = Document::parse(&markdown)
        .unwrap_or_else(|e| panic!("document failed to parse: {e:?}\n---\n{markdown}"))
        .document;
    assert!(
        parsed.cards().last().is_some_and(|c| c.kind().is_none()),
        "the appended block must parse as a kindless card"
    );
    parsed
}

fn kindless_card_stops_at_the_gate(quill_name: &str) {
    let engine = Quillmark::new();
    let quill = quillmark::quill_from_path(quills_path(quill_name))
        .unwrap_or_else(|e| panic!("{quill_name} should load: {e:?}"));
    let doc = with_kindless_card(&quill);
    let last = doc.cards().len() - 1;

    let err = engine
        .render(
            &quill,
            &doc,
            &RenderOptions::default().with_output_format(OutputFormat::Svg),
        )
        .expect_err("a kindless card must not reach the backend");

    let diag = err
        .diagnostics()
        .iter()
        .find(|d| d.code.as_deref() == Some("validation::unknown_card"))
        .unwrap_or_else(|| {
            panic!(
                "expected validation::unknown_card from {quill_name}; got: {:?}",
                err.diagnostics()
                    .iter()
                    .map(|d| &d.code)
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(
        diag.path.as_deref(),
        Some(format!("cards[{last}]").as_str()),
        "the diagnostic must anchor at the kindless card's bare index"
    );
}

#[test]
fn taro_rejects_a_kindless_card() {
    kindless_card_stops_at_the_gate("taro");
}

#[test]
fn classic_resume_rejects_a_kindless_card() {
    kindless_card_stops_at_the_gate("classic_resume");
}

#[test]
fn usaf_memo_rejects_a_kindless_card() {
    kindless_card_stops_at_the_gate("usaf_memo");
}
