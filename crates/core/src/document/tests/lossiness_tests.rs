//! Round-trip tests for comments, `!fill`, and custom tags.
//!
//! The canonical emitter preserves what taskings 01 and 02 promise:
//! top-level YAML comments round-trip as own-line comments, `!fill` tags on
//! scalar fields round-trip, and the original quoting style normalises to
//! double-quoted (type-fidelity guarantee). This file pins those contracts.

use crate::document::Document;

// ── Category: YAML comments ───────────────────────────────────────────────────

/// Top-level YAML comments survive a round-trip (tasking 01).
#[test]
fn yaml_comments_disappear_on_round_trip() {
    // Legacy name kept for git history; the assertion is now that comments
    // *do* round-trip. See tasking 01.
    let src =
        "---\nQUILL: q\n# recipient's full name\nrecipient: Jane\nauthor: Alice\n---\n\nBody.\n";

    let doc = Document::from_markdown(src).unwrap();
    let emitted = doc.to_markdown();

    assert!(
        emitted.contains("# recipient's full name"),
        "top-level YAML comment must survive round-trip\nGot:\n{}",
        emitted
    );

    // Value remains intact.
    let doc2 = Document::from_markdown(&emitted).unwrap();
    assert_eq!(
        doc2.main()
            .frontmatter()
            .get("recipient")
            .and_then(|v| v.as_str()),
        Some("Jane"),
    );

    // Comment idempotent across repeated round-trips.
    let emitted2 = doc2.to_markdown();
    assert_eq!(emitted, emitted2, "round-trip must be idempotent");
}

/// Trailing comments on value lines normalise to own-line comments on the
/// next line (opinionated canonical form per tasking 01).
#[test]
fn trailing_comments_become_own_line_on_round_trip() {
    let src = "---\nQUILL: q\ntitle: My Document # this is a comment\n---\n\nBody.\n";

    let doc = Document::from_markdown(src).unwrap();
    let emitted = doc.to_markdown();

    assert!(
        emitted.contains("# this is a comment"),
        "trailing comment text must survive\nGot:\n{}",
        emitted
    );
    assert!(
        emitted.contains("title: \"My Document\"\n# this is a comment"),
        "trailing comment must normalise to own-line on the next line\nGot:\n{}",
        emitted
    );

    // And the value is still intact.
    let doc2 = Document::from_markdown(&emitted).unwrap();
    assert_eq!(
        doc2.main()
            .frontmatter()
            .get("title")
            .and_then(|v| v.as_str()),
        Some("My Document"),
    );
}

// ── Category: Custom tags ─────────────────────────────────────────────────────

/// `!fill` tags round-trip; other custom tags are rejected with a warning
/// and the tag is dropped.
#[test]
fn custom_tags_lose_tag_but_keep_value() {
    // `!fill` case: round-trip with fill preserved.
    let src = "---\nQUILL: q\nmemo_from: !fill 2d lt example\n---\n";
    let doc = Document::from_markdown(src).unwrap();

    let fm = doc.main().frontmatter();
    assert_eq!(
        fm.get("memo_from").and_then(|v| v.as_str()),
        Some("2d lt example"),
        "string value must survive tag parsing"
    );
    assert!(fm.is_fill("memo_from"), "fill marker must be recorded");

    let emitted = doc.to_markdown();
    assert!(
        emitted.contains("memo_from: !fill"),
        "`!fill` tag must round-trip\nGot:\n{}",
        emitted
    );

    let doc2 = Document::from_markdown(&emitted).unwrap();
    assert!(
        doc2.main().frontmatter().is_fill("memo_from"),
        "fill marker must survive a full round-trip"
    );

    // Non-`!fill` tag case: warning + dropped tag.
    let src2 = "---\nQUILL: q\nmemo_from: !include value.txt\n---\n";
    let out = Document::from_markdown_with_warnings(src2).unwrap();
    assert!(
        out.warnings
            .iter()
            .any(|w| w.code.as_deref() == Some("parse::unsupported_yaml_tag")),
        "expected unsupported_yaml_tag warning; got: {:?}",
        out.warnings
    );
    let emitted2 = out.document.to_markdown();
    assert!(
        !emitted2.contains("!include"),
        "unknown tag must not re-appear on emit\nGot:\n{}",
        emitted2
    );
}

/// `!fill` on a bare key (no value) emits `key: !fill` and preserves null.
#[test]
fn fill_tag_bare_null_round_trip() {
    let src = "---\nQUILL: q\nrecipient: !fill\n---\n";

    let doc = Document::from_markdown(src).unwrap();
    let fm = doc.main().frontmatter();

    assert!(fm.get("recipient").map(|v| v.is_null()).unwrap_or(false));
    assert!(fm.is_fill("recipient"));

    let emitted = doc.to_markdown();
    assert!(
        emitted.contains("recipient: !fill\n"),
        "bare `!fill` must round-trip as `key: !fill`\nGot:\n{}",
        emitted
    );
}

/// `!fill` on every supported scalar type round-trips with the correct type.
#[test]
fn fill_tag_all_scalar_types_round_trip() {
    let src = concat!(
        "---\nQUILL: q\n",
        "s: !fill hello\n",
        "i: !fill 42\n",
        "f: !fill 3.14\n",
        "b: !fill true\n",
        "n: !fill\n",
        "---\n",
    );

    let doc = Document::from_markdown(src).unwrap();
    let fm = doc.main().frontmatter();

    assert_eq!(fm.get("s").and_then(|v| v.as_str()), Some("hello"));
    assert_eq!(fm.get("i").and_then(|v| v.as_i64()), Some(42));
    assert_eq!(fm.get("f").and_then(|v| v.as_f64()), Some(3.14));
    assert_eq!(fm.get("b").and_then(|v| v.as_bool()), Some(true));
    assert!(fm.get("n").map(|v| v.is_null()).unwrap_or(false));

    for key in ["s", "i", "f", "b", "n"] {
        assert!(fm.is_fill(key), "{} must be fill-tagged", key);
    }

    let emitted = doc.to_markdown();
    let doc2 = Document::from_markdown(&emitted).unwrap();
    for key in ["s", "i", "f", "b", "n"] {
        assert!(
            doc2.main().frontmatter().is_fill(key),
            "{} must remain fill-tagged after round-trip",
            key
        );
    }
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
    assert_eq!(
        doc2.main()
            .frontmatter()
            .get("single_q")
            .and_then(|v| v.as_str()),
        Some("hello")
    );
    assert_eq!(
        doc2.main()
            .frontmatter()
            .get("unquoted")
            .and_then(|v| v.as_str()),
        Some("world")
    );
    assert_eq!(
        doc2.main()
            .frontmatter()
            .get("double_q")
            .and_then(|v| v.as_str()),
        Some("already")
    );
}

// ── Category: Nested comments dropped ────────────────────────────────────────

#[test]
fn nested_yaml_comments_dropped_with_warning() {
    let src = "---\nQUILL: q\nitems:\n  # nested\n  - a\n  - b\n---\n";

    let out = Document::from_markdown_with_warnings(src).unwrap();
    assert!(
        out.warnings
            .iter()
            .any(|w| w.code.as_deref() == Some("parse::comments_in_nested_yaml_dropped")),
        "expected comments_in_nested_yaml_dropped warning; got: {:?}",
        out.warnings
    );

    // And the nested comment must not appear on emit.
    let emitted = out.document.to_markdown();
    assert!(
        !emitted.contains("# nested"),
        "nested comments are dropped silently\nGot:\n{}",
        emitted
    );
}
