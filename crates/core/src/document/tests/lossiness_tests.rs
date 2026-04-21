//! Lossiness documentation tests — Phase 4b.
//!
//! The canonical emitter deliberately discards certain input features that are
//! not representable in the `Document` typed model.  These tests assert the
//! **expected** loss and document it explicitly.
//!
//! Per proposal §5.4: comments, custom tags, and original quoting style are
//! all stripped on round-trip.  This is intentional, documented in
//! `Document::to_markdown`'s rustdoc, and tested here so regressions are
//! immediately visible.
//!
//! See plan §Phase 4 test item 5.

use crate::document::Document;

// ── Category: YAML comments ───────────────────────────────────────────────────

/// YAML comments (`# …`) are not stored in the `Document` model.
/// After a round-trip they do not appear in the emitted Markdown.
///
/// This is an intentional limitation (proposal §5.4 "what is lost"):
/// preserving comments would require a layout-preserving AST, which is a v2
/// feature.
#[test]
fn yaml_comments_disappear_on_round_trip() {
    // Source has a YAML comment on the title line.
    let src = "---\nQUILL: q\ntitle: My Document # this is a comment\nauthor: Alice\n---\n\nBody.\n";

    let doc = Document::from_markdown(src).unwrap();
    let emitted = doc.to_markdown();

    // The comment must not appear in the emitted output.
    assert!(
        !emitted.contains("# this is a comment"),
        "YAML comment must be stripped on emit (proposal §5.4)\nGot:\n{}",
        emitted
    );

    // The value itself must survive (comment is not part of the value).
    assert!(
        emitted.contains("My Document"),
        "value before comment must survive emit\nGot:\n{}",
        emitted
    );

    // Re-parse must succeed and value must be intact.
    let doc2 = Document::from_markdown(&emitted).unwrap();
    assert_eq!(
        doc2.frontmatter().get("title").and_then(|v| v.as_str()),
        Some("My Document"),
        "title value must survive round-trip even though comment is stripped"
    );
}

// ── Category: Custom tags ─────────────────────────────────────────────────────

/// Custom YAML tags (`!fill`, `!include`, etc.) are stripped during parsing;
/// only the scalar value is stored.  On re-emit the tag does not appear.
///
/// This is intentional (proposal §5.4): tags would require first-class
/// `QuillValue::Tagged` support, which is deferred.  The value is preserved;
/// only the tag annotation is lost.
#[test]
fn custom_tags_lose_tag_but_keep_value() {
    // `!fill` is a real Quillmark custom tag used in USAF memo templates.
    let src = "---\nQUILL: q\nmemo_from: !fill 2d lt example\n---\n";

    let doc = Document::from_markdown(src).unwrap();

    // Value must be stored as a string (tag stripped at parse time).
    let value = doc.frontmatter().get("memo_from").unwrap();
    assert!(
        value.as_str().is_some(),
        "custom-tagged value must parse as a string after tag is dropped"
    );
    assert_eq!(
        value.as_str().unwrap(),
        "2d lt example",
        "string value must survive tag stripping"
    );

    let emitted = doc.to_markdown();

    // Tag must not appear in the emission.
    assert!(
        !emitted.contains("!fill"),
        "custom tag must not reappear on emit (proposal §5.4)\nGot:\n{}",
        emitted
    );

    // Value must appear double-quoted.
    assert!(
        emitted.contains("\"2d lt example\""),
        "value must survive emit as a double-quoted string\nGot:\n{}",
        emitted
    );

    // Round-trip: value still a string.
    let doc2 = Document::from_markdown(&emitted).unwrap();
    assert_eq!(
        doc2.frontmatter().get("memo_from").and_then(|v| v.as_str()),
        Some("2d lt example"),
        "value must survive full round-trip"
    );
}

// ── Category: Original quoting style ─────────────────────────────────────────

/// The original quoting style (`'single-quoted'`, unquoted, block scalars)
/// is not preserved.  All strings are re-emitted double-quoted with JSON-style
/// escaping, regardless of how they were written in the source.
///
/// This is intentional (proposal §5.4): normalizing to double-quoted style is
/// what guarantees type fidelity for ambiguous strings like `on` and `01234`.
#[test]
fn original_quoting_style_is_not_preserved() {
    // Mix of single-quoted, unquoted, and double-quoted strings.
    let src = "---\nQUILL: q\nsingle_q: 'hello'\nunquoted: world\ndouble_q: \"already\"\n---\n";

    let doc = Document::from_markdown(src).unwrap();
    let emitted = doc.to_markdown();

    // Single-quoted must become double-quoted.
    assert!(
        !emitted.contains("'hello'"),
        "single-quoted string must not be re-emitted single-quoted\nGot:\n{}",
        emitted
    );
    assert!(
        emitted.contains("\"hello\""),
        "single-quoted string must be re-emitted double-quoted\nGot:\n{}",
        emitted
    );

    // Unquoted must become double-quoted.
    assert!(
        emitted.contains("\"world\""),
        "unquoted string must be re-emitted double-quoted\nGot:\n{}",
        emitted
    );

    // Already double-quoted is fine — stays double-quoted.
    assert!(
        emitted.contains("\"already\""),
        "double-quoted string must survive as double-quoted\nGot:\n{}",
        emitted
    );

    // Values must survive round-trip.
    let doc2 = Document::from_markdown(&emitted).unwrap();
    assert_eq!(doc2.frontmatter().get("single_q").and_then(|v| v.as_str()), Some("hello"));
    assert_eq!(doc2.frontmatter().get("unquoted").and_then(|v| v.as_str()), Some("world"));
    assert_eq!(doc2.frontmatter().get("double_q").and_then(|v| v.as_str()), Some("already"));
}
