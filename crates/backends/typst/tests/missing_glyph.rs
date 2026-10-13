//! Input text no loaded font covers warns `typst::missing_glyph`, naming each
//! character once.

use quillmark_core::backend::Backend;
use quillmark_typst::TypstBackend;

mod common;

fn missing(a: &str) -> Vec<String> {
    let quill = common::quill_with_plate(
        &common::yaml("main:\n  fields:\n    a:\n      type: string\n"),
        "#import \"@local/quillmark-helper:0.2.0\" as qm\n#qm.data.a\n",
    );
    let session = TypstBackend
        .open(&quill, &serde_json::json!({ "a": a }), common::test_date())
        .expect("the plate compiles");
    session
        .warnings()
        .iter()
        .filter(|d| d.code.as_deref() == Some("typst::missing_glyph"))
        .map(|d| d.message.clone())
        .collect()
}

#[test]
fn a_character_no_font_covers_warns_once() {
    let warnings = missing("中文 中 ok");
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(
        warnings[0],
        "no loaded font has a glyph for `中` (U+4E2D), `文` (U+6587); \
         the page draws an empty box or nothing in its place"
    );
}

#[test]
fn an_emoji_sequence_names_its_pictographs_alone() {
    let warnings = missing("👍🏽 ❤️");
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(!warnings[0].contains("U+FE0F"), "{}", warnings[0]);
}

#[test]
fn covered_text_warns_nothing() {
    assert!(missing("plain text, ünïcödé").is_empty());
}
